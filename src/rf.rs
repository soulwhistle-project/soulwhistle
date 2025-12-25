use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use crate::audio::{AudioParams, Synthesizer, SignalType};
use crate::constants::*;
use crate::utils::apply_waveform_shaping;

/// RF Worker thread that interfaces with a HackRF device to transmit signals.
///
/// NOTE: HackRF devices may not be perfectly tuned to the specified frequency out-of-the-box.
/// For precise frequency transmission, the device's crystal oscillator may need to be
/// calibrated or a more stable external clock source (like a GPSDO) should be used.
/// This implementation assumes any necessary hardware calibration has been performed.
pub struct RfWorker {
    params: Arc<Mutex<AudioParams>>,
    child: Option<Child>,
    current_freq: u64,
    current_gain: u32,
    running: bool,
    hackrf_available: bool,
    last_check_time: std::time::Instant,
    error_tx: std::sync::mpsc::Sender<String>,

    // Synthesis state for RF
    synth: Synthesizer,
    phase_accumulator: f32, // For FM
    audio_sample_rate: f32, // Track the actual audio sample rate
}

impl RfWorker {
    pub fn new(params: Arc<Mutex<AudioParams>>, error_tx: std::sync::mpsc::Sender<String>, audio_sample_rate: f32) -> Self {
        let hackrf_available = Self::check_hackrf_available();
        Self {
            params,
            child: None,
            current_freq: 0,
            current_gain: 0,
            running: false,
            hackrf_available,
            last_check_time: std::time::Instant::now(),
            synth: Synthesizer::new(audio_sample_rate),
            phase_accumulator: 0.0,
            audio_sample_rate,
            error_tx,
        }
    }
    
    fn check_hackrf_available() -> bool {
        // Check if hackrf_info command succeeds (indicates HackRF is connected)
        match Command::new("hackrf_info")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => false, // hackrf_info not installed or failed to run
        }
    }

    pub fn run(&mut self) {
        let samples_per_audio_tick = (RF_SAMPLE_RATE_HZ / self.audio_sample_rate) as usize;

        // Buffer for writing to stdout (chunked for performance)
        let mut buffer = Vec::with_capacity(RF_BUFFER_CHUNK_SIZE * 2);
        
        loop {
            // 1. Check params and manage process
            let (target_enabled, target_freq, target_gain, params_copy) = {
                let p = self.params.lock();
                (p.rf_enabled, p.rf_freq_hz, p.rf_gain, p.clone())
            };

            // Periodically re-check HackRF availability
            if self.last_check_time.elapsed() > Duration::from_secs(HACKRF_CHECK_INTERVAL_SECS) {
                self.hackrf_available = Self::check_hackrf_available();
                self.last_check_time = std::time::Instant::now();
                self.params.lock().rf_detected = self.hackrf_available;
            }

            if target_enabled {
                if !self.hackrf_available {
                    if self.running {
                        self.stop_process();
                    }
                    self.params.lock().rf_enabled = false;
                    let _ = self.error_tx.send("HackRF not detected. Please connect and restart.".to_string());
                    std::thread::sleep(Duration::from_millis(2000));
                    continue;
                }

                if !self.running
                   || target_freq != self.current_freq
                   || target_gain != self.current_gain
                {
                    self.stop_process();
                    if let Err(e) = self.start_process(target_freq, target_gain) {
                        let _ = self.error_tx.send(format!("Failed to start hackrf_transfer: {}", e));
                        self.params.lock().rf_enabled = false;
                        std::thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                    self.current_freq = target_freq;
                    self.current_gain = target_gain;
                    self.running = true;
                }
            } else {
                if self.running {
                    self.stop_process();
                }
                std::thread::sleep(Duration::from_millis(RF_WORKER_SLEEP_MS));
                continue;
            }

            // 2. Generate and write samples
            if let Some(child) = &mut self.child {
                if let Some(stdin) = child.stdin.as_mut() {
                    buffer.clear();

                    let num_audio_steps = RF_BUFFER_CHUNK_SIZE / samples_per_audio_tick;

                    for _ in 0..num_audio_steps {
                        let audio_val = self.synth.next_rf_sample(&params_copy);

                        // Apply waveform shaping based on RF pulse type
                        let shaped_val = apply_waveform_shaping(audio_val, params_copy.rf_pulse_type);
                        
                        match params_copy.rf_mode {
                            SignalType::AM => {
                                let i = (shaped_val * 127.0).clamp(-127.0, 127.0) as i8;
                                for _ in 0..samples_per_audio_tick {
                                    buffer.push(i as u8);
                                    buffer.push(0);
                                }
                            },
                            _ => { // FM Modes
                                let deviation_hz = if params_copy.rf_mode == SignalType::NBFM {
                                    NBFM_DEVIATION_HZ
                                } else {
                                    WBFM_DEVIATION_HZ
                                };

                                let dt = 1.0 / RF_SAMPLE_RATE_HZ;

                                for _ in 0..samples_per_audio_tick {
                                    let phase_delta = shaped_val * deviation_hz * 2.0 * std::f32::consts::PI * dt;
                                    self.phase_accumulator = (self.phase_accumulator + phase_delta) % (2.0 * std::f32::consts::PI);

                                    let i = (self.phase_accumulator.cos() * 127.0) as i8;
                                    let q = (self.phase_accumulator.sin() * 127.0) as i8;

                                    buffer.push(i as u8);
                                    buffer.push(q as u8);
                                }
                            }
                        }
                    }
                    
                    if stdin.write_all(&buffer).is_err() {
                        self.stop_process();
                    }
                } else {
                    self.stop_process();
                }
            }
        }
    }

    fn start_process(&mut self, freq: u64, gain: u32) -> std::io::Result<()> {
        let child = Command::new("hackrf_transfer")
            .arg("-t").arg("-")
            .arg("-f").arg(freq.to_string())
            .arg("-s").arg((RF_SAMPLE_RATE_HZ as u64).to_string())
            .arg("-a").arg("1")
            .arg("-x").arg(gain.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        self.child = Some(child);
        Ok(())
    }

    fn stop_process(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.running = false;
    }
}
