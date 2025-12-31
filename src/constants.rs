// Physical and Audio Constants

// === Frequency Constants (Hz) ===
/// Schumann resonance fundamental frequency
pub const SCHUMANN_RESONANCE_HZ: f32 = 7.83;

/// Carrier base frequency (100 Hz)
pub const CARRIER_BASE_HZ: f32 = 100.0;

/// Brycehelm variant carrier frequency (783 Hz)
pub const CARRIER_783_HZ: f32 = 783.0;

/// Solfeggio frequency - Mi (528 Hz) - DNA repair frequency
pub const SOLFEGGIO_MI_HZ: f32 = 528.0;

/// Verdi's A (432 Hz) - Natural tuning
pub const VERDI_A_HZ: f32 = 432.0;

/// Default ultrasonic ping frequency (17 kHz)
pub const DEFAULT_ULTRASONIC_PING_HZ: f32 = 17000.0;

/// Default chirp base frequency
pub const CHIRP_BASE_HZ: f32 = 2500.0;

/// Hydrogen line frequency (~1.42 GHz) - used for RF default
pub const HYDROGEN_LINE_HZ: u64 = 1_420_405_752;

// === Optimal Binaural Beat Carrier Ranges ===
/// Optimal carrier frequency for binaural beats (center of 300-600 Hz range)
pub const OPTIMAL_CARRIER_HZ: f32 = 400.0;

/// Harmonic frequency cluster (220 Hz)
pub const HARMONIC_220_HZ: f32 = 220.0;

/// High harmonic frequency (495 Hz) - Focus 12 style
pub const HARMONIC_495_HZ: f32 = 495.0;

// === Focus 10 Gamma Burst (Monroe Technique) ===
/// Interval between gamma bursts in Focus 10 mode (seconds)
pub const GAMMA_BURST_INTERVAL_SECS: f32 = 35.0;

/// Duration of each gamma burst (seconds)
pub const GAMMA_BURST_DURATION_SECS: f32 = 3.0;

/// Gamma burst carrier frequency
pub const GAMMA_CARRIER_HZ: f32 = 300.0;

/// Gamma burst binaural beat (393 Hz)
pub const GAMMA_BEAT_HZ: f32 = 393.0;

// === Sample Rates ===
/// RF transmission sample rate (2 MHz for HackRF)
pub const RF_SAMPLE_RATE_HZ: f32 = 2_000_000.0;

// === RF Modulation Constants ===
/// Narrowband FM deviation (12.5 kHz)
pub const NBFM_DEVIATION_HZ: f32 = 12_500.0;

/// Wideband FM deviation (75 kHz)
pub const WBFM_DEVIATION_HZ: f32 = 75_000.0;

// === RF Worker Configuration ===
/// HackRF detection check interval (seconds)
pub const HACKRF_CHECK_INTERVAL_SECS: u64 = 5;

/// RF buffer chunk size (samples)
pub const RF_BUFFER_CHUNK_SIZE: usize = 4096;

// === UI Adjustment Step Sizes ===
/// Binaural beat frequency adjustment multiplier
pub const BEAT_ADJUST_MULTIPLIER: f32 = 10.0;

/// Binaural beat frequency range (Hz)
pub const BEAT_MIN_HZ: f32 = 0.1;
pub const BEAT_MAX_HZ: f32 = 30.0;

/// Ping frequency coarse adjustment step (Hz)
pub const PING_FREQ_COARSE_STEP: f32 = 1000.0;

/// Ping frequency fine adjustment step (Hz)
pub const PING_FREQ_FINE_STEP: f32 = 100.0;

/// Ping frequency range (Hz)
pub const PING_FREQ_MIN_HZ: f32 = 1000.0;
pub const PING_FREQ_MAX_HZ: f32 = 100000.0;

/// RF frequency adjustment step (100 kHz)
pub const RF_FREQ_STEP_HZ: u64 = 100_000;

/// RF gain step (dB)
pub const RF_GAIN_STEP_DB: u32 = 1;

/// RF gain maximum (dB)
pub const RF_GAIN_MAX_DB: u32 = 47;

/// Stream port coarse adjustment step
pub const PORT_COARSE_STEP: u16 = 100;

/// Stream port fine adjustment step
pub const PORT_FINE_STEP: u16 = 10;

/// Stream port range
pub const PORT_MIN: u16 = 1024;
pub const PORT_MAX: u16 = 65535;

// === UI Display Constants ===
/// Description text maximum width (characters)
pub const DESC_MAX_WIDTH: usize = 90;

/// Approximate character count for 3 lines of description
pub const DESC_BRIEF_CHAR_THRESHOLD: usize = 270;

/// Number of lines to show in brief description mode
pub const DESC_BRIEF_LINES: usize = 3;

/// Status message timeout (seconds) for normal messages
pub const STATUS_TIMEOUT_SECS: u64 = 3;

/// Status message timeout (seconds) for warnings
pub const STATUS_WARNING_TIMEOUT_SECS: u64 = 8;

/// Header offset for mixer display (number of header lines)
pub const MIXER_HEADER_OFFSET: usize = 9;

// === Session Timing Defaults (Monroe-style 30-minute session) ===
/// Default startup phase duration (minutes)
pub const DEFAULT_STARTUP_DURATION_MIN: f32 = 2.0;

/// Default induction phase duration (minutes)
pub const DEFAULT_INDUCTION_DURATION_MIN: f32 = 13.0;

/// Default stabilization phase duration (minutes)
pub const DEFAULT_STABILIZATION_DURATION_MIN: f32 = 10.0;

/// Default return phase duration (minutes)
pub const DEFAULT_RETURN_DURATION_MIN: f32 = 5.0;

// === Audio Processing ===
/// Default master volume (start low for safety)
pub const DEFAULT_MASTER_VOLUME: f32 = 0.1;

/// Default coherence/binaural volume
pub const DEFAULT_COHERENCE_VOLUME: f32 = 0.5;

/// Default custom binaural beat frequency (Hz)
pub const DEFAULT_CUSTOM_BINAURAL_HZ: f32 = 4.1;

/// Default streaming buffer duration (milliseconds)
pub const STREAM_BUFFER_DURATION_MS: u32 = 2000;

/// Default streaming port
pub const DEFAULT_STREAM_PORT: u16 = 1123;

/// Default RF gain (dB)
pub const DEFAULT_RF_GAIN_DB: u32 = 30;

// === Chirp Timing ===
/// Chirp repeat period (seconds)
pub const CHIRP_PERIOD_SECS: f32 = 10.0;

/// Chirp duration (seconds)
pub const CHIRP_DURATION_SECS: f32 = 0.2;

/// Chirp FM modulation factor
pub const CHIRP_FM_MOD_FACTOR: f32 = 20.0;

/// Chirp FM modulation range (Hz)
pub const CHIRP_FM_MOD_RANGE_HZ: f32 = 50.0;

/// Chirp frequency sweep range for synthetic chirps (Hz)
pub const CHIRP_SWEEP_START_HZ: f32 = 2000.0;
pub const CHIRP_SWEEP_END_HZ: f32 = 3000.0;

// === Breath Layer ===
/// Breath LFO frequency (Hz)
pub const BREATH_LFO_HZ: f32 = 0.2;

// === Noise Reduction ===
/// Pink noise approximation factor
pub const PINK_NOISE_FACTOR: f32 = 0.8;

// === Harmonic Mixing ===
/// Harmonic 220Hz volume relative to carrier
pub const HARMONIC_220_RELATIVE_VOL: f32 = 0.15;

/// Harmonic 495Hz volume relative to carrier
pub const HARMONIC_495_RELATIVE_VOL: f32 = 0.1;

// === Streaming ===
/// Streaming read sample count per chunk
pub const STREAM_READ_CHUNK_SIZE: usize = 1024;

/// Streaming read wait time when buffer empty (ms)
pub const STREAM_READ_WAIT_MS: u64 = 5;

/// WAV file format constants
pub const WAV_INFINITE_SIZE: u32 = 0xFFFFFFFF;
pub const WAV_PCM_FORMAT: u16 = 1;
pub const WAV_STEREO_CHANNELS: u16 = 2;
pub const WAV_BITS_PER_SAMPLE: u16 = 16;
pub const WAV_BYTES_PER_SAMPLE: u16 = 2;
pub const WAV_BLOCK_ALIGN: u16 = WAV_STEREO_CHANNELS * WAV_BYTES_PER_SAMPLE;

// === File Paths ===
/// Saved preset filename
pub const PRESET_FILENAME: &str = "preset.json";

/// Default preset to load on startup
pub const DEFAULT_PRESET_FILENAME: &str = "DEFAULT_deep_focus_active.json";

/// Get user config directory for presets
/// Returns ~/.config/soulwhistle/presets on Linux
/// Returns ~/Library/Application Support/soulwhistle/presets on macOS
/// Returns %APPDATA%\soulwhistle\presets on Windows
pub fn get_presets_dir() -> std::path::PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "soulwhistle") {
        proj_dirs.config_dir().join("presets")
    } else {
        // Fallback to current directory if we can't determine config dir
        std::path::PathBuf::from("presets")
    }
}

// === Amplitude Thresholds ===
/// Threshold for AM modulation depth
pub const AM_MODULATION_MIN: f32 = 0.5;

/// FM modulation range for Schumann FM mode
pub const FM_MODULATION_RANGE_HZ: f32 = 20.0;

// === Signal Clipping ===
/// PCM i16 maximum value
pub const PCM_I16_MAX: f32 = 32767.0;

/// Audio sample clamp range
pub const AUDIO_CLAMP_MIN: f32 = -1.0;
pub const AUDIO_CLAMP_MAX: f32 = 1.0;

// === Brainwave State Boundaries (Hz) ===
pub const DELTA_MAX_HZ: f32 = 4.0;
pub const THETA_MAX_HZ: f32 = 8.0;
pub const ALPHA_MAX_HZ: f32 = 12.0;
pub const BETA_MAX_HZ: f32 = 30.0;
// Gamma: > 30 Hz

// === Polling and Timing ===
/// Event polling interval (milliseconds)
pub const EVENT_POLL_INTERVAL_MS: u64 = 50;

/// RF worker sleep interval when disabled (milliseconds)
pub const RF_WORKER_SLEEP_MS: u64 = 100;
