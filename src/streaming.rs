use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use crate::audio::AudioParams;
use crate::constants::*;

/// Internal ring state guarded by a single lock.
/// `write_count` is a monotonic count of all samples ever pushed, so a reader can
/// hold an absolute read index that survives `pop_front()` rotation of the deque.
struct RingState {
    samples: VecDeque<(f32, f32)>,
    write_count: u64,
}

/// Circular buffer for stereo audio samples
pub struct AudioRingBuffer {
    state: Mutex<RingState>,
    capacity: usize,
    sample_rate: u32,
    stream_epoch: AtomicU32, // Incremented when preset changes to force client reconnects
}

impl AudioRingBuffer {
    pub fn new(sample_rate: u32, buffer_ms: u32) -> Self {
        let capacity = (sample_rate * buffer_ms / 1000) as usize;
        Self {
            state: Mutex::new(RingState {
                samples: VecDeque::with_capacity(capacity),
                write_count: 0,
            }),
            capacity,
            sample_rate,
            stream_epoch: AtomicU32::new(0),
        }
    }

    /// Push multiple stereo samples into the ring buffer (batch operation)
    /// Single lock acquisition for entire batch - critical for realtime audio.
    /// Uses `try_lock` so the real-time audio callback never blocks waiting on a
    /// streaming reader. On contention the whole batch is dropped and `write_count`
    /// does not advance, so readers never see the gap: listeners hear a short
    /// discontinuity (a click) instead of the audio device glitching for everyone.
    pub fn push_samples_batch(&self, samples: &[(f32, f32)]) {
        let Some(mut st) = self.state.try_lock() else {
            return;
        };
        for &(left, right) in samples {
            if st.samples.len() >= self.capacity {
                st.samples.pop_front(); // Remove oldest sample
            }
            st.samples.push_back((left, right));
            st.write_count += 1;
        }
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Flush the buffer (clear all samples) and increment epoch to force client reconnects.
    ///
    /// Called from the real-time audio callback, so it never blocks: returns
    /// `false` if a streaming reader holds the lock, and the caller retries on the
    /// next callback. The epoch only moves once the clear has actually happened.
    pub fn try_flush(&self) -> bool {
        let Some(mut st) = self.state.try_lock() else {
            return false;
        };
        st.samples.clear();
        // Keep write_count monotonic so readers don't see the index jump backwards.
        drop(st);

        // Increment epoch to signal preset change to active streaming clients
        self.stream_epoch.fetch_add(1, Ordering::Release);
        true
    }

    /// Absolute read position `lead` samples behind the newest sample, clamped to
    /// the oldest sample still held. New listeners start here, near live.
    pub fn live_position(&self, lead: u64) -> u64 {
        let st = self.state.lock();
        let oldest = st.write_count - st.samples.len() as u64;
        st.write_count.saturating_sub(lead).max(oldest)
    }

    /// Get current stream epoch (for detecting preset changes in streaming clients)
    pub fn get_epoch(&self) -> u32 {
        self.stream_epoch.load(Ordering::Acquire)
    }

    /// Read up to `count` new samples starting at the reader's absolute `position`.
    ///
    /// `position` is an absolute sample index into the lifetime stream (not a deque
    /// index), so it stays correct as the producer rotates the deque via `pop_front`.
    /// If the reader has fallen further behind than the buffer holds, it is
    /// fast-forwarded to the oldest available sample (dropping the gap) to stay live.
    pub fn read_samples(&self, position: &mut u64, count: usize) -> Vec<(f32, f32)> {
        let st = self.state.lock();
        let len = st.samples.len() as u64;
        let write_count = st.write_count;
        let oldest = write_count - len; // absolute index of samples.front()

        // Reader fell behind the retained window: skip the gap, resume at oldest.
        if *position < oldest {
            *position = oldest;
        }

        // Nothing new available yet.
        if *position >= write_count {
            return Vec::new();
        }

        let start = (*position - oldest) as usize;
        let end = (start + count).min(st.samples.len());
        let samples: Vec<(f32, f32)> = st.samples.range(start..end).copied().collect();
        *position += (end - start) as u64;

        samples
    }
}

/// HTTP streaming server for audio
pub struct StreamingServer {
    buffer: Arc<AudioRingBuffer>,
    port: u16,
    pub client_count: Arc<Mutex<usize>>,
    params: Arc<Mutex<AudioParams>>,
}

impl StreamingServer {
    pub fn new(
        buffer: Arc<AudioRingBuffer>,
        port: u16,
        client_count: Arc<Mutex<usize>>,
        params: Arc<Mutex<AudioParams>>,
    ) -> Self {
        Self {
            buffer,
            port,
            client_count,
            params,
        }
    }

    /// Start the HTTP server (blocking call - run in separate thread).
    /// Returns when streaming is disabled or the port changes, so the bound
    /// socket is dropped and the supervisor can rebind cleanly.
    pub fn run(self) {
        let server = match tiny_http::Server::http(format!("0.0.0.0:{}", self.port)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to start streaming server on port {}: {}", self.port, e);
                return;
            }
        };

        loop {
            // Poll with a timeout so we can periodically check whether to shut down.
            let request = match server.recv_timeout(Duration::from_millis(STREAM_ACCEPT_TIMEOUT_MS)) {
                Ok(Some(req)) => req,
                Ok(None) => {
                    // Timed out with no request: re-check shutdown condition.
                    if self.should_stop() {
                        return; // Drops `server`, freeing the port for rebind.
                    }
                    continue;
                }
                Err(e) => {
                    eprintln!("Streaming server recv error: {}", e);
                    return;
                }
            };

            let url = request.url().to_string();
            let (path, query) = match url.split_once('?') {
                Some((p, q)) => (p.to_string(), q.to_string()),
                None => (url.clone(), String::new()),
            };

            // Accept /stream.wav, /stream.flac or just /
            let is_valid_path = path == "/stream.wav" || path == "/stream.flac" || path == "/";
            let default_format = if path == "/stream.flac" { StreamFormat::Flac24 } else { StreamFormat::S16 };

            let format = match StreamFormat::from_query(&query, default_format) {
                Ok(f) => f,
                Err(bad) => {
                    let response = tiny_http::Response::from_string(format!(
                        "Unknown format '{}'. Use ?fmt=s16 (default), s24, f32, flac (24-bit) or flac16.\n", bad))
                        .with_status_code(400);
                    let _ = request.respond(response);
                    continue;
                }
            };

            if is_valid_path {
                // Clone necessary data for the thread
                let buffer = self.buffer.clone();
                let client_count = self.client_count.clone();
                let title = self.params.lock().preset_title.clone()
                    .unwrap_or_else(|| "Soulwhistle".to_string());

                // Increment client count
                {
                    let mut count = client_count.lock();
                    *count += 1;
                }

                // Spawn thread to handle this client
                std::thread::spawn(move || {
                    if let Err(e) = handle_stream_request(request, buffer.clone(), format, &title) {
                        eprintln!("Stream error: {}", e);
                    }

                    // Decrement client count when done
                    let mut count = client_count.lock();
                    *count = count.saturating_sub(1);
                });
            } else {
                // Unknown path - return 404
                let response = tiny_http::Response::from_string("404 Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
            }

            if self.should_stop() {
                return;
            }
        }
    }

    /// True when streaming has been disabled or retargeted to a different port.
    fn should_stop(&self) -> bool {
        let p = self.params.lock();
        !p.stream_enabled || p.stream_port != self.port
    }
}

/// Handle a single stream request
fn handle_stream_request(
    request: tiny_http::Request,
    buffer: Arc<AudioRingBuffer>,
    format: StreamFormat,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = buffer.sample_rate();

    // Header for an endless stream
    let (header, content_type) = if format.is_flac() {
        (create_flac_header(sample_rate, format, title)?, &b"audio/flac"[..])
    } else {
        (create_wav_header(sample_rate, format), &b"audio/wav"[..])
    };

    // Create a streaming reader, starting near live
    let stream_reader = AudioStreamReader::new(buffer, Some(header), format);

    // Create response
    let response = tiny_http::Response::new(
        tiny_http::StatusCode(200),
        vec![
            tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type).unwrap(),
            tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap(),
        ],
        stream_reader,
        None, // Unknown length (infinite stream)
        None,
    );
    
    request.respond(response)?;
    
    Ok(())
}

/// Sample format of the HTTP stream, chosen with `?fmt=` on the stream URL.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamFormat {
    /// 16-bit PCM with TPDF dither. Default: every browser and player accepts it.
    S16,
    /// 24-bit PCM with TPDF dither. VLC, mpv, ffplay, Chrome.
    S24,
    /// 32-bit IEEE float: bit-exact copy of the synth output. VLC, mpv, ffplay, Chrome.
    F32,
    /// FLAC, 16-bit with TPDF dither: lossless at roughly half the WAV bandwidth.
    Flac16,
    /// FLAC, 24-bit with TPDF dither. Default for `/stream.flac`.
    Flac24,
}

impl StreamFormat {
    /// Parse the `fmt` query parameter; `Err` carries the unrecognised value.
    fn from_query(query: &str, default: Self) -> Result<Self, String> {
        for pair in query.split('&') {
            if let Some(v) = pair.strip_prefix("fmt=") {
                return match v.to_ascii_lowercase().as_str() {
                    "s16" | "16" => Ok(Self::S16),
                    "s24" | "24" => Ok(Self::S24),
                    "f32" | "float" => Ok(Self::F32),
                    "flac" | "flac24" => Ok(Self::Flac24),
                    "flac16" => Ok(Self::Flac16),
                    _ => Err(v.to_string()),
                };
            }
        }
        Ok(default)
    }

    fn is_flac(self) -> bool {
        matches!(self, Self::Flac16 | Self::Flac24)
    }

    /// Bytes per sample in the WAV formats; bit depth / 8 for FLAC.
    fn bytes_per_sample(self) -> u16 {
        match self {
            Self::S16 | Self::Flac16 => 2,
            Self::S24 | Self::Flac24 => 3,
            Self::F32 => 4,
        }
    }
}

/// Per-listener FLAC encoder state. Encoding runs on the listener's HTTP thread,
/// never in the audio callback.
struct FlacEncoder {
    config: flacenc::error::Verified<flacenc::config::Encoder>,
    info: flacenc::component::StreamInfo,
    framebuf: flacenc::source::FrameBuf,
    block: Vec<i32>, // Interleaved L/R samples for the block being filled
    frame_number: usize,
}

impl FlacEncoder {
    fn new(sample_rate: u32, bits: usize) -> std::io::Result<Self> {
        use flacenc::error::Verify;
        let err = |e: String| std::io::Error::new(std::io::ErrorKind::Other, e);
        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|(_, e)| err(format!("FLAC config: {e:?}")))?;
        let info = flac_stream_info(sample_rate, bits)?;
        let framebuf = flacenc::source::FrameBuf::with_size(2, FLAC_BLOCK_SIZE)
            .map_err(|e| err(format!("FLAC buffer: {e:?}")))?;
        Ok(Self {
            config,
            info,
            framebuf,
            block: Vec::with_capacity(FLAC_BLOCK_SIZE * 2),
            frame_number: 0,
        })
    }

    /// Encode the full block into `out` as one FLAC frame.
    fn encode_block(&mut self, out: &mut Vec<u8>) -> std::io::Result<()> {
        use flacenc::component::BitRepr;
        use flacenc::source::Fill;
        let err = |e: String| std::io::Error::new(std::io::ErrorKind::Other, e);
        self.framebuf
            .fill_interleaved(&self.block)
            .map_err(|e| err(format!("FLAC fill: {e:?}")))?;
        let frame = flacenc::encode_fixed_size_frame(&self.config, &self.framebuf, self.frame_number, &self.info)
            .map_err(|e| err(format!("FLAC encode: {e:?}")))?;
        let mut sink = flacenc::bitsink::ByteSink::new();
        frame.write(&mut sink).map_err(|e| err(format!("FLAC write: {e:?}")))?;
        out.extend_from_slice(sink.as_slice());
        self.frame_number += 1;
        self.block.clear();
        Ok(())
    }
}

/// STREAMINFO for an endless stereo stream with fixed-size blocks.
fn flac_stream_info(sample_rate: u32, bits: usize) -> std::io::Result<flacenc::component::StreamInfo> {
    let err = |e: String| std::io::Error::new(std::io::ErrorKind::Other, e);
    let mut info = flacenc::component::StreamInfo::new(sample_rate as usize, 2, bits)
        .map_err(|e| err(format!("FLAC stream info: {e:?}")))?;
    info.set_block_sizes(FLAC_BLOCK_SIZE, FLAC_BLOCK_SIZE)
        .map_err(|e| err(format!("FLAC block size: {e:?}")))?;
    // Frame sizes 0 = unknown. flacenc's defaults (min 0xFFFFFF, max 0) are
    // placeholders a file encoder overwrites at the end; sent live, VLC took the
    // 16 MB "minimum frame size" literally and never output a frame.
    info.set_frame_sizes(0, 0)
        .map_err(|e| err(format!("FLAC frame size: {e:?}")))?;
    // total_samples stays 0 (unknown: endless stream); MD5 stays zero (not checked).
    Ok(info)
}

/// Custom reader that streams audio data
struct AudioStreamReader {
    buffer: Arc<AudioRingBuffer>,
    position: u64,
    header: Option<Vec<u8>>,
    pcm_buffer: Vec<u8>, // Reusable buffer for PCM conversion
    epoch: u32,          // Last seen epoch (bumped on preset change)
    format: StreamFormat,
    dither_state: u32,   // xorshift32 state for TPDF dither
    lead: u64,           // Samples behind live to (re)start at
    flac: Option<FlacEncoder>,
    pending: Vec<u8>,    // Encoded bytes not yet handed to the HTTP writer
    pending_pos: usize,
}

impl AudioStreamReader {
    fn new(buffer: Arc<AudioRingBuffer>, header: Option<Vec<u8>>, format: StreamFormat) -> Self {
        let lead = buffer.sample_rate() as u64 * STREAM_START_LEAD_MS / 1000;
        let position = buffer.live_position(lead);
        let epoch = buffer.get_epoch();
        let flac = if format.is_flac() {
            FlacEncoder::new(buffer.sample_rate(), 8 * format.bytes_per_sample() as usize).ok()
        } else {
            None
        };
        Self {
            buffer,
            position,
            header,
            pcm_buffer: Vec::new(),
            epoch,
            format,
            dither_state: 0x9E37_79B9,
            lead,
            flac,
            pending: Vec::new(),
            pending_pos: 0,
        }
    }

    /// Block until up to `max` new samples are available. Never returns empty:
    /// returning Ok(0) from `read` would make players treat the stream as ended.
    fn next_samples(&mut self, max: usize) -> Vec<(f32, f32)> {
        loop {
            // Preset changed: the buffer was flushed. Jump to the new preset's audio
            // (the usual lead behind live, clamped to the first sample after the
            // flush) and keep the connection open: closing it made VLC and browsers
            // stop playing.
            let epoch = self.buffer.get_epoch();
            if epoch != self.epoch {
                self.epoch = epoch;
                self.position = self.buffer.live_position(self.lead);
            }

            let samples = self.buffer.read_samples(&mut self.position, max);
            if !samples.is_empty() {
                return samples;
            }

            // No data yet (buffer underrun) — wait briefly, then retry.
            std::thread::sleep(Duration::from_millis(STREAM_READ_WAIT_MS));
        }
    }

    /// Fill one FLAC block from the ring buffer and encode it into `pending`.
    fn fill_flac_frame(&mut self) -> std::io::Result<()> {
        let mut enc = self.flac.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "FLAC encoder unavailable")
        })?;
        let full_scale = if self.format == StreamFormat::Flac16 { PCM_I16_MAX } else { PCM_I24_MAX };
        while enc.block.len() < FLAC_BLOCK_SIZE * 2 {
            let want = FLAC_BLOCK_SIZE - enc.block.len() / 2;
            for (l, r) in self.next_samples(want) {
                let li = self.quantise(l, full_scale);
                let ri = self.quantise(r, full_scale);
                enc.block.push(li);
                enc.block.push(ri);
            }
        }
        self.pending.clear();
        self.pending_pos = 0;
        let result = enc.encode_block(&mut self.pending);
        self.flac = Some(enc);
        result
    }

    /// Triangular (TPDF) dither in LSBs: sum of two uniform values, range (-1, 1).
    /// Removes the signal-correlated distortion that plain truncation adds.
    #[inline(always)]
    fn tpdf(&mut self) -> f32 {
        let mut next = || {
            let mut x = self.dither_state;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.dither_state = x;
            (x >> 8) as f32 / (1u32 << 24) as f32
        };
        next() - next()
    }

    /// Quantise one sample to an integer of `full_scale` with TPDF dither.
    #[inline(always)]
    fn quantise(&mut self, x: f32, full_scale: f32) -> i32 {
        let v = x.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX) * full_scale + self.tpdf();
        v.round().clamp(-full_scale - 1.0, full_scale) as i32
    }
}

impl std::io::Read for AudioStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // First, send the header if we have one
        if let Some(header) = self.header.take() {
            let len = header.len().min(buf.len());
            buf[..len].copy_from_slice(&header[..len]);

            // If we couldn't fit the whole header, put the rest back
            if len < header.len() {
                self.header = Some(header[len..].to_vec());
            }

            return Ok(len);
        }

        // FLAC: hand out the current encoded frame, encoding the next when empty.
        if self.format.is_flac() {
            if self.pending_pos >= self.pending.len() {
                self.fill_flac_frame()?;
            }
            let rest = &self.pending[self.pending_pos..];
            let len = rest.len().min(buf.len());
            buf[..len].copy_from_slice(&rest[..len]);
            self.pending_pos += len;
            return Ok(len);
        }

        // Only read as many samples as fit in `buf`, so the whole PCM batch is
        // always flushed, never truncated and lost.
        let frame_bytes = 2 * self.format.bytes_per_sample() as usize;
        let max_samples = (buf.len() / frame_bytes).min(STREAM_READ_CHUNK_SIZE);
        if max_samples == 0 {
            // Caller-provided buffer can't hold a full stereo frame; signal EOF.
            return Ok(0);
        }

        let samples = self.next_samples(max_samples);

        // Reuse the PCM buffer (clear but keep capacity)
        let mut pcm = std::mem::take(&mut self.pcm_buffer);
        pcm.clear();
        pcm.reserve(samples.len() * frame_bytes);

        for (left, right) in samples {
            for x in [left, right] {
                match self.format {
                    StreamFormat::S16 => {
                        let v = self.quantise(x, PCM_I16_MAX) as i16;
                        pcm.extend_from_slice(&v.to_le_bytes());
                    }
                    StreamFormat::S24 => {
                        let v = self.quantise(x, PCM_I24_MAX);
                        pcm.extend_from_slice(&v.to_le_bytes()[..3]);
                    }
                    StreamFormat::F32 => {
                        pcm.extend_from_slice(&x.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX).to_le_bytes());
                    }
                    StreamFormat::Flac16 | StreamFormat::Flac24 => unreachable!("handled above"),
                }
            }
        }
        self.pcm_buffer = pcm;

        // PCM batch is sized to fit `buf` by construction, so this copies all of it.
        let len = self.pcm_buffer.len().min(buf.len());
        buf[..len].copy_from_slice(&self.pcm_buffer[..len]);

        Ok(len)
    }
}

/// `fLaC` marker, STREAMINFO for an endless stream, and a Vorbis comment block
/// carrying the preset title (shown by players as the track title); FLAC frames
/// follow. Players read tags once, when they connect.
fn create_flac_header(sample_rate: u32, format: StreamFormat, title: &str) -> std::io::Result<Vec<u8>> {
    use flacenc::component::BitRepr;
    let info = flac_stream_info(sample_rate, 8 * format.bytes_per_sample() as usize)?;
    let mut stream = flacenc::component::Stream::with_stream_info(info);
    stream.add_metadata_block(flacenc::component::MetadataBlockData::new_unknown(
        FLAC_VORBIS_COMMENT_TYPE,
        &vorbis_comment(&[("TITLE", title), ("ARTIST", "Soulwhistle")]),
    ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("FLAC tags: {e:?}")))?);
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("FLAC header: {e:?}"))
    })?;
    Ok(sink.as_slice().to_vec())
}

/// FLAC metadata block type 4: VORBIS_COMMENT.
const FLAC_VORBIS_COMMENT_TYPE: u8 = 4;

/// Vorbis comment block body: vendor string, then `KEY=value` pairs, all with
/// 32-bit little-endian lengths (the one little-endian structure in FLAC).
fn vorbis_comment(tags: &[(&str, &str)]) -> Vec<u8> {
    let vendor = concat!("soulwhistle ", env!("CARGO_PKG_VERSION"));
    let mut out = Vec::new();
    out.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    out.extend_from_slice(vendor.as_bytes());
    out.extend_from_slice(&(tags.len() as u32).to_le_bytes());
    for (key, value) in tags {
        let entry = format!("{key}={value}");
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(entry.as_bytes());
    }
    out
}

/// Create a WAV file header for an endless stream in the given format.
fn create_wav_header(sample_rate: u32, format: StreamFormat) -> Vec<u8> {
    let bytes = format.bytes_per_sample();
    let block_align = WAV_STEREO_CHANNELS * bytes;
    let (format_tag, fmt_size) = match format {
        // IEEE float needs the 18-byte fmt chunk (with cbSize = 0).
        StreamFormat::F32 => (WAV_IEEE_FLOAT_FORMAT, 18u32),
        _ => (WAV_PCM_FORMAT, 16u32),
    };
    let mut header = Vec::with_capacity(46);

    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&WAV_INFINITE_SIZE.to_le_bytes());
    header.extend_from_slice(b"WAVE");

    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&fmt_size.to_le_bytes());
    header.extend_from_slice(&format_tag.to_le_bytes());
    header.extend_from_slice(&WAV_STEREO_CHANNELS.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&(sample_rate * block_align as u32).to_le_bytes());
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&(bytes * 8).to_le_bytes());
    if fmt_size == 18 {
        header.extend_from_slice(&0u16.to_le_bytes()); // cbSize
    }

    // data chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&WAV_INFINITE_SIZE.to_le_bytes());

    header
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn buffer_with(samples: usize) -> Arc<AudioRingBuffer> {
        let b = Arc::new(AudioRingBuffer::new(48000, 2000));
        let data: Vec<(f32, f32)> = (0..samples).map(|i| (i as f32 / samples as f32, -0.5)).collect();
        b.push_samples_batch(&data);
        b
    }

    /// Encode about 0.4 s of a known signal through the FLAC stream path and, when the
    /// reference `flac` tool is installed, decode it and compare every sample.
    #[test]
    fn flac_stream_decodes_to_the_same_samples() {
        for (fmt, full_scale) in [(StreamFormat::Flac24, PCM_I24_MAX), (StreamFormat::Flac16, PCM_I16_MAX)] {
            let fs = 48000u32;
            let frames = FLAC_BLOCK_SIZE * 5;
            let b = Arc::new(AudioRingBuffer::new(fs, 2000));
            let src: Vec<(f32, f32)> = (0..frames)
                .map(|i| {
                    let t = i as f32 / fs as f32;
                    (0.5 * (2.0 * std::f32::consts::PI * 400.0 * t).sin(),
                     0.5 * (2.0 * std::f32::consts::PI * 410.0 * t).sin())
                })
                .collect();
            b.push_samples_batch(&src);

            let header = create_flac_header(fs, fmt, "Test preset").unwrap();
            assert_eq!(&header[..4], b"fLaC");
            // STREAMINFO: block sizes fixed, frame sizes 0 (unknown). A non-zero
            // minimum frame size stalls VLC on a live stream.
            assert_eq!(&header[8..12], &[0x10, 0x00, 0x10, 0x00], "block sizes");
            assert_eq!(&header[12..18], &[0u8; 6], "frame sizes must be 0 (unknown)");
            let mut r = AudioStreamReader::new(b, Some(header), fmt);
            r.position = 0; // start at the first sample for the comparison
            let mut bytes = Vec::new();
            let mut buf = [0u8; 8192];
            // Header, then the first 4 full blocks (the 5th would block for more audio).
            while r.flac.as_ref().map_or(0, |e| e.frame_number) < 4 || r.pending_pos < r.pending.len() {
                let n = r.read(&mut buf).unwrap();
                bytes.extend_from_slice(&buf[..n]);
            }

            let dir = std::env::temp_dir().join(format!("soulwhistle-flac-{}-{:?}", std::process::id(), fmt));
            std::fs::create_dir_all(&dir).unwrap();
            let flac_path = dir.join("stream.flac");
            let wav_path = dir.join("decoded.wav");
            std::fs::write(&flac_path, &bytes).unwrap();
            let decoded = std::process::Command::new("flac")
                .args(["-d", "-f", "-s", "-o"]).arg(&wav_path).arg(&flac_path)
                .status();
            match decoded {
                Ok(st) if st.success() => {}
                Ok(st) => panic!("{fmt:?}: flac could not decode the stream ({st})"),
                Err(_) => {
                    eprintln!("flac tool not installed: decode check skipped");
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                }
            }
            let tags = std::process::Command::new("metaflac")
                .args(["--show-tag=TITLE", "--show-tag=ARTIST"]).arg(&flac_path)
                .output();
            if let Ok(out) = tags {
                let text = String::from_utf8_lossy(&out.stdout);
                assert!(text.contains("TITLE=Test preset"), "{fmt:?} tags: {text}");
                assert!(text.contains("ARTIST=Soulwhistle"), "{fmt:?} tags: {text}");
            }
            let wav = std::fs::read(&wav_path).unwrap();
            let data = &wav[wav.len() - FLAC_BLOCK_SIZE * 4 * 2 * fmt.bytes_per_sample() as usize..];
            let bps = fmt.bytes_per_sample() as usize;
            for (i, frame) in data.chunks(2 * bps).enumerate() {
                let dec = |c: &[u8]| -> f32 {
                    let v = if bps == 2 { i16::from_le_bytes([c[0], c[1]]) as i32 }
                            else { i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8 };
                    v as f32 / full_scale
                };
                let tol = 2.0 / full_scale;
                assert!((dec(&frame[..bps]) - src[i].0).abs() <= tol, "{fmt:?} left sample {i}");
                assert!((dec(&frame[bps..]) - src[i].1).abs() <= tol, "{fmt:?} right sample {i}");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn query_selects_format() {
        assert_eq!(StreamFormat::from_query("", StreamFormat::S16), Ok(StreamFormat::S16));
        assert_eq!(StreamFormat::from_query("fmt=f32", StreamFormat::S16), Ok(StreamFormat::F32));
        assert_eq!(StreamFormat::from_query("x=1&fmt=S24", StreamFormat::S16), Ok(StreamFormat::S24));
        assert!(StreamFormat::from_query("fmt=mp3", StreamFormat::S16).is_err());
        assert_eq!(StreamFormat::from_query("fmt=flac", StreamFormat::S16), Ok(StreamFormat::Flac24));
        assert_eq!(StreamFormat::from_query("fmt=flac16", StreamFormat::S16), Ok(StreamFormat::Flac16));
        assert_eq!(StreamFormat::from_query("", StreamFormat::Flac24), Ok(StreamFormat::Flac24));
    }

    #[test]
    fn headers_describe_each_format() {
        for (fmt, tag, bits, len) in [
            (StreamFormat::S16, 1u16, 16u16, 44usize),
            (StreamFormat::S24, 1, 24, 44),
            (StreamFormat::F32, 3, 32, 46),
        ] {
            let h = create_wav_header(48000, fmt);
            assert_eq!(h.len(), len);
            assert_eq!(u16::from_le_bytes([h[20], h[21]]), tag);
            assert_eq!(u16::from_le_bytes([h[34], h[35]]), bits);
            let block = u16::from_le_bytes([h[32], h[33]]);
            assert_eq!(block, 2 * bits / 8);
            assert_eq!(u32::from_le_bytes(h[28..32].try_into().unwrap()), 48000 * block as u32);
            assert_eq!(&h[len - 8..len - 4], b"data");
        }
    }

    #[test]
    fn new_listener_starts_near_live() {
        let b = buffer_with(48000); // 1 s buffered
        let r = AudioStreamReader::new(b, None, StreamFormat::S16);
        let lead = 48000 * STREAM_START_LEAD_MS / 1000;
        assert_eq!(r.position, 48000 - lead);
    }

    #[test]
    fn stream_survives_preset_change() {
        let b = buffer_with(4800);
        let mut r = AudioStreamReader::new(b.clone(), None, StreamFormat::F32);
        let mut buf = [0u8; 4096];
        assert!(r.read(&mut buf).unwrap() > 0);

        // Preset change: flush, then new audio arrives.
        assert!(b.try_flush());
        b.push_samples_batch(&[(0.25, 0.75); 480]);
        let n = r.read(&mut buf).unwrap();
        assert!(n > 0, "connection must stay open across a preset change");
        // Reader jumped to the new audio, not leftovers from the old preset.
        let l = f32::from_le_bytes(buf[0..4].try_into().unwrap());
        let rr = f32::from_le_bytes(buf[4..8].try_into().unwrap());
        assert_eq!((l, rr), (0.25, 0.75));
    }

    #[test]
    fn f32_is_bit_exact_and_ints_are_close() {
        let vals = [(0.123456f32, -0.987654f32); 64];
        for fmt in [StreamFormat::S16, StreamFormat::S24, StreamFormat::F32] {
            let b = Arc::new(AudioRingBuffer::new(48000, 2000));
            b.push_samples_batch(&vals);
            let mut r = AudioStreamReader::new(b, None, fmt);
            let mut buf = [0u8; 4096];
            let n = r.read(&mut buf).unwrap();
            let bps = fmt.bytes_per_sample() as usize;
            assert_eq!(n, 64 * 2 * bps);
            let decode = |c: &[u8]| -> f32 {
                match fmt {
                    StreamFormat::S16 => i16::from_le_bytes([c[0], c[1]]) as f32 / PCM_I16_MAX,
                    StreamFormat::S24 => (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / PCM_I24_MAX,
                    StreamFormat::F32 => f32::from_le_bytes(c.try_into().unwrap()),
                    StreamFormat::Flac16 | StreamFormat::Flac24 => unreachable!(),
                }
            };
            let tol = match fmt {
                StreamFormat::S16 => 2.0 / PCM_I16_MAX,
                StreamFormat::S24 => 2.0 / PCM_I24_MAX,
                StreamFormat::F32 => 0.0,
                StreamFormat::Flac16 | StreamFormat::Flac24 => unreachable!(),
            };
            for frame in buf[..n].chunks(2 * bps) {
                assert!((decode(&frame[..bps]) - vals[0].0).abs() <= tol, "{fmt:?} left");
                assert!((decode(&frame[bps..]) - vals[0].1).abs() <= tol, "{fmt:?} right");
            }
        }
    }
}
