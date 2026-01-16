use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use crate::constants::*;

/// Circular buffer for stereo audio samples
pub struct AudioRingBuffer {
    buffer: Mutex<VecDeque<(f32, f32)>>,
    capacity: usize,
    sample_rate: u32,
    stream_epoch: Mutex<u32>, // Incremented when preset changes to force client reconnects
}

impl AudioRingBuffer {
    pub fn new(sample_rate: u32, buffer_ms: u32) -> Self {
        let capacity = (sample_rate * buffer_ms / 1000) as usize;
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            sample_rate,
            stream_epoch: Mutex::new(0),
        }
    }

    /// Push multiple stereo samples into the ring buffer (batch operation)
    /// Single lock acquisition for entire batch - critical for realtime audio
    pub fn push_samples_batch(&self, samples: &[(f32, f32)]) {
        let mut buf = self.buffer.lock();
        for &(left, right) in samples {
            if buf.len() >= self.capacity {
                buf.pop_front(); // Remove oldest sample
            }
            buf.push_back((left, right));
        }
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Flush the buffer (clear all samples) and increment epoch to force client reconnects
    pub fn flush(&self) {
        let mut buf = self.buffer.lock();
        buf.clear();
        drop(buf);

        // Increment epoch to signal preset change to active streaming clients
        let mut epoch = self.stream_epoch.lock();
        *epoch = epoch.wrapping_add(1);
    }

    /// Get current stream epoch (for detecting preset changes in streaming clients)
    pub fn get_epoch(&self) -> u32 {
        *self.stream_epoch.lock()
    }

    /// Read samples from a specific position
    pub fn read_samples(&self, position: &mut usize, count: usize) -> Vec<(f32, f32)> {
        let buf = self.buffer.lock();
        let available = buf.len();
        
        if available == 0 {
            return Vec::new();
        }

        // If position is too far behind, reset to start
        if *position >= available {
            *position = 0;
        }

        let end = (*position + count).min(available);
        let samples: Vec<(f32, f32)> = buf.range(*position..end).copied().collect();
        *position = end;
        
        samples
    }
}

/// HTTP streaming server for audio
pub struct StreamingServer {
    buffer: Arc<AudioRingBuffer>,
    port: u16,
    pub client_count: Arc<Mutex<usize>>,
}

impl StreamingServer {
    pub fn new(buffer: Arc<AudioRingBuffer>, port: u16, client_count: Arc<Mutex<usize>>) -> Self {
        Self {
            buffer,
            port,
            client_count,
        }
    }

    /// Start the HTTP server (blocking call - run in separate thread)
    pub fn run(self) {
        let server = match tiny_http::Server::http(format!("0.0.0.0:{}", self.port)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to start streaming server on port {}: {}", self.port, e);
                return;
            }
        };

        // Server started successfully - silently run
        for request in server.incoming_requests() {
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
        }
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
    position: usize,
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
        // Check if epoch changed (preset changed) - close connection to force client reconnect
        if self.buffer.get_epoch() != self.initial_epoch {
            // Return 0 to signal end-of-stream, causing client to reconnect
            return Ok(0);
        }

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

        // Read samples from buffer
        let samples = self.buffer.read_samples(&mut self.position, STREAM_READ_CHUNK_SIZE);

        if samples.is_empty() {
            // No data yet, wait a bit then try again
            std::thread::sleep(std::time::Duration::from_millis(STREAM_READ_WAIT_MS));
            return Ok(0);
        }

        // Reuse the PCM buffer (clear but keep capacity)
        self.pcm_buffer.clear();
        self.pcm_buffer.reserve(samples.len() * 4);

        for (left, right) in samples {
            let left_i16 = (left.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX) * PCM_I16_MAX) as i16;
            let right_i16 = (right.clamp(AUDIO_CLAMP_MIN, AUDIO_CLAMP_MAX) * PCM_I16_MAX) as i16;

            self.pcm_buffer.extend_from_slice(&left_i16.to_le_bytes());
            self.pcm_buffer.extend_from_slice(&right_i16.to_le_bytes());
        }

        // Copy to output buffer
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
