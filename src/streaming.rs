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
    /// Single lock acquisition for entire batch - critical for realtime audio
    pub fn push_samples_batch(&self, samples: &[(f32, f32)]) {
        let mut st = self.state.lock();
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

    /// Flush the buffer (clear all samples) and increment epoch to force client reconnects
    pub fn flush(&self) {
        {
            let mut st = self.state.lock();
            st.samples.clear();
            // Keep write_count monotonic so readers don't see the index jump backwards.
        }

        // Increment epoch to signal preset change to active streaming clients
        self.stream_epoch.fetch_add(1, Ordering::Release);
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

            let path = request.url().to_string();

            // Accept /stream.wav or just /
            let is_valid_path = path == "/stream.wav" || path == "/";

            if is_valid_path {
                // Clone necessary data for the thread
                let buffer = self.buffer.clone();
                let client_count = self.client_count.clone();

                // Increment client count
                {
                    let mut count = client_count.lock();
                    *count += 1;
                }

                // Spawn thread to handle this client
                std::thread::spawn(move || {
                    if let Err(e) = handle_stream_request(request, buffer.clone()) {
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
) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = buffer.sample_rate();
    let initial_epoch = buffer.get_epoch(); // Capture epoch at connection start

    // Create WAV header for infinite stream
    let wav_header = create_wav_header(sample_rate);

    // Create a streaming reader
    let stream_reader = AudioStreamReader::new(buffer, Some(wav_header), initial_epoch);
    
    // Create response
    let response = tiny_http::Response::new(
        tiny_http::StatusCode(200),
        vec![
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"audio/wav"[..]).unwrap(),
            tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap(),
        ],
        stream_reader,
        None, // Unknown length (infinite stream)
        None,
    );
    
    request.respond(response)?;
    
    Ok(())
}

/// Custom reader that streams audio data
struct AudioStreamReader {
    buffer: Arc<AudioRingBuffer>,
    position: u64,
    header: Option<Vec<u8>>,
    pcm_buffer: Vec<u8>, // Reusable buffer for PCM conversion
    initial_epoch: u32,  // Epoch when connection started (to detect preset changes)
}

impl AudioStreamReader {
    fn new(buffer: Arc<AudioRingBuffer>, header: Option<Vec<u8>>, initial_epoch: u32) -> Self {
        Self {
            buffer,
            position: 0,
            header,
            pcm_buffer: Vec::new(),
            initial_epoch,
        }
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

        // Only read as many samples as fit in `buf` (4 bytes per stereo frame), so the
        // whole PCM batch is always flushed — never truncated and lost.
        let max_samples = (buf.len() / 4).min(STREAM_READ_CHUNK_SIZE);
        if max_samples == 0 {
            // Caller-provided buffer can't hold a full stereo frame; signal EOF.
            return Ok(0);
        }

        // Block until data is available, retrying on transient underrun. Returning
        // Ok(0) here would be interpreted as end-of-stream and close the connection,
        // so it is reserved exclusively for the preset-change reconnect below.
        let samples = loop {
            // Epoch changed (preset changed): close connection to force client reconnect.
            if self.buffer.get_epoch() != self.initial_epoch {
                return Ok(0);
            }

            let samples = self.buffer.read_samples(&mut self.position, max_samples);
            if !samples.is_empty() {
                break samples;
            }

            // No data yet (buffer underrun) — wait briefly, then retry.
            std::thread::sleep(Duration::from_millis(STREAM_READ_WAIT_MS));
        };

        // Reuse the PCM buffer (clear but keep capacity)
        self.pcm_buffer.clear();
        self.pcm_buffer.reserve(samples.len() * 4);

        for (left, right) in samples {
            let left_i16 = (left.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX) * PCM_I16_MAX) as i16;
            let right_i16 = (right.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX) * PCM_I16_MAX) as i16;

            self.pcm_buffer.extend_from_slice(&left_i16.to_le_bytes());
            self.pcm_buffer.extend_from_slice(&right_i16.to_le_bytes());
        }

        // PCM batch is sized to fit `buf` by construction, so this copies all of it.
        let len = self.pcm_buffer.len().min(buf.len());
        buf[..len].copy_from_slice(&self.pcm_buffer[..len]);

        Ok(len)
    }
}

/// Create a WAV file header for streaming
fn create_wav_header(sample_rate: u32) -> Vec<u8> {
    let mut header = Vec::new();

    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&WAV_INFINITE_SIZE.to_le_bytes());
    header.extend_from_slice(b"WAVE");

    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    header.extend_from_slice(&WAV_PCM_FORMAT.to_le_bytes());
    header.extend_from_slice(&WAV_STEREO_CHANNELS.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&(sample_rate * WAV_BLOCK_ALIGN as u32).to_le_bytes());
    header.extend_from_slice(&WAV_BLOCK_ALIGN.to_le_bytes());
    header.extend_from_slice(&WAV_BITS_PER_SAMPLE.to_le_bytes());

    // data chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&WAV_INFINITE_SIZE.to_le_bytes());

    header
}
