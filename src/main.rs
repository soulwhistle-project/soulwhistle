use std::error::Error;
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};
use std::sync::Arc;
use parking_lot::Mutex;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod audio;
mod coherence;
mod dsp;
mod rf;
mod streaming;
mod constants;
mod utils;
mod embedded_presets;

use audio::{AudioParams, Synthesizer, SignalType};
use coherence::BeingType;
use rf::RfWorker;
use streaming::{AudioRingBuffer, StreamingServer};
use constants::*;
use utils::{wrap_text, cycle_index};

enum AppMode {
    Mixer,
    PresetSelect,
    PitchMatch,
}

#[derive(Clone, Copy, PartialEq)]
enum PresetDescMode {
    Hidden,     // Collapsed - no description shown
    Brief,      // Show first 3 lines with "..."
    Full,       // Show full description
}

struct PresetInfo {
    filename: String,
    title: Option<String>,
    description: Option<String>,
    experimental: Option<bool>,
}

/// Message sent from the real-time audio callback to the recording writer thread.
enum RecordingMsg {
    Data(Vec<u8>),
    Stop,
}

/// Number of PCM buffers circulating between the audio callback and the writer.
/// Each holds one callback's worth of samples; the callback can run this many
/// callbacks ahead of the disk before it starts dropping blocks.
const REC_POOL_BUFFERS: usize = 32;
/// Initial capacity of each pooled buffer: 8192 stereo i16 frames (32 KiB), larger
/// than any callback size cpal normally delivers, so filling it never reallocates.
const REC_BUFFER_BYTES: usize = 8192 * WAV_BLOCK_ALIGN as usize;

/// Shared state for WAV recording between audio callback and UI.
///
/// Real-time rules: the audio callback never allocates, never blocks and never
/// touches the disk for recording. Buffers are pre-allocated in a pool; the
/// callback takes a free one (`pool_rx`), fills it and hands it to the writer
/// thread with a non-blocking `try_send` on a bounded channel; the writer writes
/// it to disk and returns it to the pool. If the writer falls behind and no buffer
/// is free, the callback drops that block and counts it in `dropped_blocks`.
struct RecordingState {
    active: bool,
    tx: Option<std::sync::mpsc::SyncSender<RecordingMsg>>,
    pool_rx: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
    writer: Option<std::thread::JoinHandle<()>>,
    dropped_blocks: Arc<std::sync::atomic::AtomicU64>,
    filepath: Option<String>,
}

impl RecordingState {
    fn new() -> Self {
        Self {
            active: false,
            tx: None,
            pool_rx: None,
            writer: None,
            dropped_blocks: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            filepath: None,
        }
    }
}

/// Stop the active recording, if any, and wait until the WAV file is complete
/// (header patched, buffers flushed). Returns the file path and the number of
/// blocks dropped because the writer fell behind.
///
/// Must be called before the process exits, or the writer thread is killed
/// mid-file and the header keeps its zero placeholder sizes.
fn stop_recording(recording: &Mutex<RecordingState>) -> Option<(String, u64)> {
    let (tx, writer, path, dropped) = {
        let mut rec = recording.lock();
        if !rec.active {
            return None;
        }
        rec.active = false;
        rec.pool_rx = None;
        (
            rec.tx.take(),
            rec.writer.take(),
            rec.filepath.take().unwrap_or_default(),
            rec.dropped_blocks.swap(0, std::sync::atomic::Ordering::Relaxed),
        )
    }; // lock released before waiting, so the audio callback is never held up

    if let Some(tx) = tx {
        // Blocking send is fine here: this is the UI thread, and the writer is
        // draining the channel.
        let _ = tx.send(RecordingMsg::Stop);
    }
    if let Some(handle) = writer {
        let _ = handle.join();
    }
    Some((path, dropped))
}

/// Owns the WAV file and performs all disk I/O for one recording session.
/// Runs on its own thread; the audio callback only ever sends it buffers, which
/// are returned through `pool_tx` once written.
fn run_recording_writer(
    mut writer: std::io::BufWriter<File>,
    rx: std::sync::mpsc::Receiver<RecordingMsg>,
    pool_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
) {
    // u64 so a long session cannot overflow the counter; the WAV header itself
    // is 32-bit, so sizes are clamped there (files over 4 GiB are not valid WAV).
    let mut bytes_written: u64 = 0;
    for msg in rx {
        match msg {
            RecordingMsg::Data(buf) => {
                if writer.write_all(&buf).is_ok() {
                    bytes_written += buf.len() as u64;
                }
                let _ = pool_tx.try_send(buf);
            }
            RecordingMsg::Stop => break,
        }
    }

    // Finalize the WAV header now that the real size is known.
    let data_size = bytes_written.min((u32::MAX - 36) as u64) as u32;
    let file_size = data_size + 36; // 44 byte header - 8 byte RIFF preamble
    if writer.seek(SeekFrom::Start(4)).is_ok() {
        let _ = writer.write_all(&file_size.to_le_bytes());
    }
    if writer.seek(SeekFrom::Start(40)).is_ok() {
        let _ = writer.write_all(&data_size.to_le_bytes());
    }
    let _ = writer.flush();
}

struct App {
    mode: AppMode,
    params: Arc<Mutex<AudioParams>>,
    channels: Vec<ChannelInfo>,
    state: ListState,

    // Preset state
    status_msg: Option<(String, std::time::Instant)>,
    preset_list: Vec<PresetInfo>,
    preset_state: ListState,
    current_preset: Option<String>,

    // Network streaming
    stream_client_count: Arc<Mutex<usize>>,

    // Collapsible sections
    signal_layer_collapsed: bool,
    hackrf_collapsed: bool,
    streaming_collapsed: bool,
    tinnitus_collapsed: bool,
    preset_desc_mode: PresetDescMode,

    // Mapping from UI index to original channel index (for collapse toggle)
    visible_channel_indices: Vec<usize>,

    // RF safety
    rf_disclaimer_shown: bool,

    // WAV recording
    recording: Arc<Mutex<RecordingState>>,
    // Output device sample rate, for the above-Nyquist warning on the ping layer
    sample_rate: f32,
}

struct ChannelInfo {
    name: String,
    // We will use an identifier to map to the params struct
    id: ChannelId,
}

#[derive(Clone, Copy)]
enum ChannelId {
    Master,
    // Being/Consciousness Selection
    BeingType,
    PresetDescription,
    CoherenceVol,
    BinauralAdjust,
    SessionTimer,
    // UAP/Unknown Frequencies
    Carrier,
    Harmonic,
    Ping,
    PingFreq,
    Chirp,
    Pad,
    Breath,
    // Tinnitus Notched Sound Therapy
    NotchVol,
    TinnitusPitch,
    NotchWidth,
    NotchSource,
    // RF Controls
    RfEnable,
    RfFreq,
    RfGain,
    RfMode,
    RfPulseType,
    // Network Streaming
    StreamEnable,
    StreamPort,
}

impl App {
    fn new(params: Arc<Mutex<AudioParams>>, stream_client_count: Arc<Mutex<usize>>, recording: Arc<Mutex<RecordingState>>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));

        let mut preset_state = ListState::default();
        preset_state.select(Some(0));
        
        Self {
            mode: AppMode::Mixer,
            params,
            signal_layer_collapsed: false,  // Start expanded for visibility
            hackrf_collapsed: true,         // Start collapsed (advanced)
            streaming_collapsed: true,      // Start collapsed (experimental)
            tinnitus_collapsed: true,       // Start collapsed (specialized therapy)
            preset_desc_mode: PresetDescMode::Hidden,  // Start hidden to save space
            visible_channel_indices: Vec::new(),
            rf_disclaimer_shown: false,
            channels: vec![
                ChannelInfo { name: "Master Volume".to_string(), id: ChannelId::Master },
                ChannelInfo { name: "PRESET:".to_string(), id: ChannelId::BeingType },
                ChannelInfo { name: "PRESET_DESC".to_string(), id: ChannelId::PresetDescription },
                ChannelInfo { name: "BINAURAL BEATS".to_string(), id: ChannelId::CoherenceVol },
                ChannelInfo { name: "  Volume".to_string(), id: ChannelId::CoherenceVol },
                ChannelInfo { name: "  Beat Adjust".to_string(), id: ChannelId::BinauralAdjust },
                ChannelInfo { name: "  Session Progress".to_string(), id: ChannelId::SessionTimer },
                ChannelInfo { name: "SIGNAL LAYER".to_string(), id: ChannelId::Carrier },
                ChannelInfo { name: "  Carrier (7.83Hz)".to_string(), id: ChannelId::Carrier },
                ChannelInfo { name: "  Harmonic (528Hz)".to_string(), id: ChannelId::Harmonic },
                ChannelInfo { name: "  Ultrasonic Ping".to_string(), id: ChannelId::Ping },
                ChannelInfo { name: "  Ping Frequency".to_string(), id: ChannelId::PingFreq },
                ChannelInfo { name: "  Organic Chirps".to_string(), id: ChannelId::Chirp },
                ChannelInfo { name: "  Ambient Pad (432Hz)".to_string(), id: ChannelId::Pad },
                ChannelInfo { name: "  Breath Layer".to_string(), id: ChannelId::Breath },
                ChannelInfo { name: "TINNITUS THERAPY [NOTCHED NOISE]".to_string(), id: ChannelId::NotchVol },
                ChannelInfo { name: "  Notched Noise Vol".to_string(), id: ChannelId::NotchVol },
                ChannelInfo { name: "  Tinnitus Pitch".to_string(), id: ChannelId::TinnitusPitch },
                ChannelInfo { name: "  Notch Width".to_string(), id: ChannelId::NotchWidth },
                ChannelInfo { name: "  Notch Source".to_string(), id: ChannelId::NotchSource },
                ChannelInfo { name: "HACKRF TRANSMIT".to_string(), id: ChannelId::RfEnable },
                ChannelInfo { name: "  RF Enable".to_string(), id: ChannelId::RfEnable },
                ChannelInfo { name: "  RF Frequency".to_string(), id: ChannelId::RfFreq },
                ChannelInfo { name: "  RF Gain (VGA)".to_string(), id: ChannelId::RfGain },
                ChannelInfo { name: "  RF Modulation Mode".to_string(), id: ChannelId::RfMode },
                ChannelInfo { name: "  RF Pulse Waveform".to_string(), id: ChannelId::RfPulseType },
                ChannelInfo { name: "NETWORK STREAMING [EXPERIMENTAL]".to_string(), id: ChannelId::StreamEnable },
                ChannelInfo { name: "  Stream Enable".to_string(), id: ChannelId::StreamEnable },
                ChannelInfo { name: "  Stream Port".to_string(), id: ChannelId::StreamPort },
            ],
            state,
            status_msg: None,
            preset_list: Vec::new(),
            preset_state,
            current_preset: None,
            stream_client_count,
            recording,
            sample_rate: 0.0,
        }
    }

    // --- Helper Methods ---

    /// Check if a channel is locked (signal layer locked when lock_signal_layer is true)
    fn is_channel_locked(&self, id: ChannelId, params: &AudioParams) -> bool {
        let is_signal_channel = matches!(id,
            ChannelId::Carrier | ChannelId::Harmonic | ChannelId::Ping |
            ChannelId::Chirp | ChannelId::Pad | ChannelId::Breath);

        is_signal_channel && params.lock_signal_layer
    }

    // --- Navigation ---
    fn next(&mut self) {
        match self.mode {
            AppMode::Mixer => {
                if self.visible_channel_indices.is_empty() { return; }
                let i = cycle_index(self.state.selected().unwrap_or(0), self.visible_channel_indices.len(), 1);
                self.state.select(Some(i));
            },
            AppMode::PresetSelect => {
                if self.preset_list.is_empty() { return; }
                let i = cycle_index(self.preset_state.selected().unwrap_or(0), self.preset_list.len(), 1);
                self.preset_state.select(Some(i));
            }
            AppMode::PitchMatch => {}
        }
    }

    fn previous(&mut self) {
        match self.mode {
            AppMode::Mixer => {
                if self.visible_channel_indices.is_empty() { return; }
                let i = cycle_index(self.state.selected().unwrap_or(0), self.visible_channel_indices.len(), -1);
                self.state.select(Some(i));
            },
            AppMode::PresetSelect => {
                if self.preset_list.is_empty() { return; }
                let i = cycle_index(self.preset_state.selected().unwrap_or(0), self.preset_list.len(), -1);
                self.preset_state.select(Some(i));
            }
            AppMode::PitchMatch => {}
        }
    }

    fn adjust_volume(&mut self, delta: f32) {
        if let Some(visual_idx) = self.state.selected() {
            // Map visual index to actual channel index
            if visual_idx >= self.visible_channel_indices.len() {
                return; // Out of bounds
            }
            let channel_idx = self.visible_channel_indices[visual_idx];
            let id = self.channels[channel_idx].id;
            let params_arc = self.params.clone();
            let mut params = params_arc.lock();

            // Don't allow adjustments on locked signal channels
            if self.is_channel_locked(id, &params) {
                return;
            }
            
            match id {
                ChannelId::Master => params.master_vol = (params.master_vol + delta).clamp(0.0, 1.0),
                
                ChannelId::BeingType => {
                    // Cycle through ACTUAL presets list with left/right arrows
                    
                    // Ensure list is populated
                    if self.preset_list.is_empty() {
                        self.refresh_presets();
                    }
                    
                    if self.preset_list.is_empty() { return; }

                    // Find current index
                    let current_idx = if let Some(ref current_name) = self.current_preset {
                        self.preset_list.iter().position(|p| &p.filename == current_name)
                    } else {
                        // Try to find a match based on current being type
                        let default_filename = params.coherence.being_type.default_preset_filename();
                        self.preset_list.iter().position(|preset| preset.filename == default_filename)
                    };

                    let idx = current_idx.unwrap_or(0);
                    
                    let next_idx = if delta > 0.0 {
                        if idx >= self.preset_list.len() - 1 { 0 } else { idx + 1 }
                    } else {
                        if idx == 0 { self.preset_list.len() - 1 } else { idx - 1 }
                    };

                    // Load the new preset
                    let preset_info = &self.preset_list[next_idx];
                    let filename = preset_info.filename.clone();

                    drop(params); // Release lock before file I/O

                    // Use hybrid loading (user dir first, then embedded fallback)
                    if let Some(json) = load_preset_hybrid(&filename) {
                        if let Ok(mut loaded) = serde_json::from_str::<AudioParams>(&json) {
                            loaded.rf_enabled = false; // Safety

                            // Preserve streaming settings when loading preset
                            let mut params = self.params.lock();
                            let stream_enabled = params.stream_enabled;
                            let stream_port = params.stream_port;
                            let current_version = params.preset_version;

                            loaded.stream_enabled = stream_enabled;
                            loaded.stream_port = stream_port;
                            loaded.preset_version = current_version.wrapping_add(1);

                            *params = loaded;
                            drop(params);

                            self.current_preset = Some(filename.clone());
                            return;
                        }
                    }

                    // If load fails
                    self.status_msg = Some((format!("Failed to load {}", filename), std::time::Instant::now()));
                },
                ChannelId::CoherenceVol => {
                    // Only allow adjustment when preset selected (binaural beats for human listening need headphones!)
                    if !matches!(params.coherence.being_type, BeingType::Unknown) {
                        params.coherence.volume = (params.coherence.volume + delta).clamp(0.0, 1.0);
                    }
                },
                ChannelId::BinauralAdjust => {
                    // Only works in Custom mode
                    if matches!(params.coherence.being_type, BeingType::HumanCustom) {
                        let new_hz = (params.coherence.custom_binaural_hz + delta * BEAT_ADJUST_MULTIPLIER).clamp(BEAT_MIN_HZ, BEAT_MAX_HZ);
                        params.coherence.apply_custom_binaural(new_hz);
                    }
                },

                ChannelId::SessionTimer | ChannelId::PresetDescription => {
                    // Read-only display, no adjustment
                },

                ChannelId::PingFreq => {
                    let step = if delta.abs() > 0.05 { PING_FREQ_COARSE_STEP } else { PING_FREQ_FINE_STEP };
                    let new_hz = (params.ping_freq_hz + delta.signum() * step).clamp(PING_FREQ_MIN_HZ, PING_FREQ_MAX_HZ);
                    params.ping_freq_hz = new_hz;
                },

                ChannelId::Carrier => params.carrier_vol = (params.carrier_vol + delta).clamp(0.0, 1.0),
                ChannelId::Harmonic => params.harmonic_vol = (params.harmonic_vol + delta).clamp(0.0, 1.0),
                ChannelId::Ping => params.ping_vol = (params.ping_vol + delta).clamp(0.0, 1.0),
                ChannelId::Chirp => params.chirp_vol = (params.chirp_vol + delta).clamp(0.0, 1.0),
                ChannelId::Pad => params.pad_vol = (params.pad_vol + delta).clamp(0.0, 1.0),
                ChannelId::Breath => params.breath_vol = (params.breath_vol + delta).clamp(0.0, 1.0),

                ChannelId::NotchVol => params.notch_vol = (params.notch_vol + delta).clamp(0.0, 1.0),
                ChannelId::TinnitusPitch => {
                    let step = if delta.abs() > 0.05 { TINNITUS_FREQ_COARSE_STEP } else { TINNITUS_FREQ_FINE_STEP };
                    params.tinnitus_freq_hz = (params.tinnitus_freq_hz + delta.signum() * step)
                        .clamp(TINNITUS_FREQ_MIN_HZ, TINNITUS_FREQ_MAX_HZ);
                },
                ChannelId::NotchWidth => {
                    params.notch_width_oct = (params.notch_width_oct + delta.signum() * NOTCH_WIDTH_STEP_OCT)
                        .clamp(NOTCH_WIDTH_MIN_OCT, NOTCH_WIDTH_MAX_OCT);
                },
                ChannelId::NotchSource => {
                    // Toggle pink/white with arrows (also cycled via 'o')
                    params.notch_use_pink = delta > 0.0;
                },

                ChannelId::RfEnable => {
                    // Show disclaimer first time user tries to enable RF
                    if delta > 0.0 && !params.rf_enabled && !self.rf_disclaimer_shown {
                        self.rf_disclaimer_shown = true;
                        self.status_msg = Some((
                            "⚠️  RF DISCLAIMER: Verify frequency is legal in your jurisdiction! Press → again to enable".to_string(),
                            std::time::Instant::now()
                        ));
                        return; // Don't enable yet
                    }

                    // Toggle if delta is significant
                    if delta > 0.0 {
                        params.rf_enabled = true;
                        if !params.rf_detected {
                            self.status_msg = Some((
                                "⚠️  RF enabled but NO HACKRF DETECTED - nothing will transmit".to_string(),
                                std::time::Instant::now()
                            ));
                        }
                    } else if delta < 0.0 {
                        params.rf_enabled = false;
                    }
                },
                ChannelId::RfFreq => {
                    if delta > 0.0 {
                        params.rf_freq_hz = params.rf_freq_hz.saturating_add(RF_FREQ_STEP_HZ);
                    } else {
                        params.rf_freq_hz = params.rf_freq_hz.saturating_sub(RF_FREQ_STEP_HZ);
                    }
                },
                ChannelId::RfGain => {
                    if delta > 0.0 {
                        params.rf_gain = (params.rf_gain + RF_GAIN_STEP_DB).min(RF_GAIN_MAX_DB);
                    } else {
                        params.rf_gain = params.rf_gain.saturating_sub(RF_GAIN_STEP_DB);
                    }
                },
                ChannelId::RfMode | ChannelId::RfPulseType => {
                    // These use 'm' key to cycle, not arrows
                    // Arrows do nothing
                 }
                
                ChannelId::StreamEnable => {
                    // Toggle with space, not arrows
                }
                
                ChannelId::StreamPort => {
                    let step = if delta.abs() > 0.05 { PORT_COARSE_STEP } else { PORT_FINE_STEP };
                    let new_port = if delta > 0.0 {
                        params.stream_port.saturating_add(step)
                    } else {
                        params.stream_port.saturating_sub(step)
                    };
                    params.stream_port = new_port.clamp(PORT_MIN, PORT_MAX);
                }

            }
        }
    }

    fn toggle_volume(volume: &mut f32) {
        if *volume > 0.0 {
            *volume = 0.0;
        } else {
            *volume = 0.5;
        }
    }
 
     fn toggle_playback(&mut self) {
         let mut params = self.params.lock();
         // Context sensitive toggle based on selected channel
         if let Some(visual_idx) = self.state.selected() {
             if visual_idx < self.visible_channel_indices.len() {
                 let channel_idx = self.visible_channel_indices[visual_idx];
                 match self.channels[channel_idx].id {
                     ChannelId::RfEnable => {
                         params.rf_enabled = !params.rf_enabled;
                         return;
                     }
                     ChannelId::StreamEnable => {
                         params.stream_enabled = !params.stream_enabled;
                         return;
                     }
                     _ => {}
                 }
             }
         }
         params.playing = !params.playing;
    }
    
    fn toggle_mute(&mut self) {
        if let Some(visual_idx) = self.state.selected() {
            if visual_idx >= self.visible_channel_indices.len() {
                return;
            }
            let channel_idx = self.visible_channel_indices[visual_idx];
            let id = self.channels[channel_idx].id;
            let mut params = self.params.lock();

            if params.lock_signal_layer {
                let is_signal_channel = matches!(id,
                    ChannelId::Carrier | ChannelId::Harmonic | ChannelId::Ping |
                    ChannelId::Chirp | ChannelId::Pad | ChannelId::Breath);
                if is_signal_channel {
                    return; // Don't allow mute on locked signal channels
                }
            }

            match id {
                ChannelId::Master => Self::toggle_volume(&mut params.master_vol),
                ChannelId::CoherenceVol => Self::toggle_volume(&mut params.coherence.volume),
                ChannelId::Carrier => Self::toggle_volume(&mut params.carrier_vol),
                ChannelId::Harmonic => Self::toggle_volume(&mut params.harmonic_vol),
                ChannelId::Ping => Self::toggle_volume(&mut params.ping_vol),
                ChannelId::Chirp => Self::toggle_volume(&mut params.chirp_vol),
                ChannelId::Pad => Self::toggle_volume(&mut params.pad_vol),
                ChannelId::Breath => Self::toggle_volume(&mut params.breath_vol),
                ChannelId::NotchVol => Self::toggle_volume(&mut params.notch_vol),
                _ => {} // Other channels don't support mute
            }
        }
    }

    fn cycle_modulation(&mut self) {
        if let Some(visual_idx) = self.state.selected() {
             if visual_idx >= self.visible_channel_indices.len() {
                 return;
             }
             let channel_idx = self.visible_channel_indices[visual_idx];
             let id = self.channels[channel_idx].id;
             let mut params = self.params.lock();

             // Don't allow modulation changes on locked signal channels
             if self.is_channel_locked(id, &params) {
                 return;
             }

             match id {
                 ChannelId::Carrier => {
                     params.carrier_type = match params.carrier_type {
                         SignalType::SchumannAM => SignalType::SchumannFM,
                         SignalType::SchumannFM => SignalType::Schumann783AM,
                         SignalType::Schumann783AM => SignalType::Sine100Hz,
                         SignalType::Sine100Hz => SignalType::Square,
                         _ => SignalType::SchumannAM,
                     };
                 },
                 ChannelId::Harmonic => {
                     params.harmonic_type = match params.harmonic_type {
                         SignalType::Sine => SignalType::Triangle,
                         SignalType::Triangle => SignalType::Square,
                         SignalType::Square => SignalType::Saw,
                         _ => SignalType::Sine,
                     };
                 },
                 ChannelId::Ping => {
                     params.ping_type = match params.ping_type {
                         SignalType::Sine => SignalType::Square,
                         SignalType::Square => SignalType::Saw,
                         SignalType::Saw => SignalType::GammaPulse,
                         _ => SignalType::Sine,
                     };
                 },
                 ChannelId::Chirp => {
                     params.chirp_type = match params.chirp_type {
                         SignalType::OrganicChirp => SignalType::SyntheticChirp,
                         SignalType::SyntheticChirp => SignalType::Saw, // 8-bit style
                         SignalType::Saw => SignalType::Square,
                         _ => SignalType::OrganicChirp,
                     };
                 },
                 ChannelId::Pad => {
                     params.pad_type = match params.pad_type {
                         SignalType::Sine => SignalType::Triangle,
                         SignalType::Triangle => SignalType::Saw, // Harsh pad
                         _ => SignalType::Sine,
                     };
                 },
                 ChannelId::Breath => {
                     params.breath_type = match params.breath_type {
                         SignalType::LfoBreathing => SignalType::WhiteNoise,
                         SignalType::WhiteNoise => SignalType::PinkNoise,
                         SignalType::PinkNoise => SignalType::Sine, // Drone
                         _ => SignalType::LfoBreathing,
                     };
                 },
                 ChannelId::NotchSource => {
                     params.notch_use_pink = !params.notch_use_pink;
                 },
                  ChannelId::RfMode => {
                      // Cycle RF Modulation Mode
                      params.rf_mode = match params.rf_mode {
                          SignalType::WBFM => SignalType::NBFM,
                          SignalType::NBFM => SignalType::AM,
                          _ => SignalType::WBFM,
                      };
                  },
                  ChannelId::RfPulseType => {
                      // Cycle RF Pulse Waveform
                      params.rf_pulse_type = match params.rf_pulse_type {
                          SignalType::Sine => SignalType::Triangle,
                          SignalType::Triangle => SignalType::Square,
                          SignalType::Square => SignalType::Saw,
                          _ => SignalType::Sine,
                      };
                  },
                  ChannelId::RfFreq | ChannelId::RfEnable | ChannelId::RfGain => {
                      // No modulation cycling for these
                  },
                 _ => {}
             }
        }
    }

    fn refresh_presets(&mut self) {
        self.preset_list.clear();
        let presets_dir = get_presets_dir();
        let _ = std::fs::create_dir_all(&presets_dir);

        use std::collections::HashSet;
        let mut loaded_files = HashSet::new();

        // Load from user directory first (user presets and modified defaults)
        if let Ok(entries) = std::fs::read_dir(&presets_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.ends_with(".json") {
                        loaded_files.insert(name.clone());

                        // Try to load metadata
                        let path = presets_dir.join(&name);
                        let mut title = None;
                        let mut description = None;
                        let mut experimental = None;

                        if let Ok(mut file) = File::open(&path) {
                            use std::io::Read;
                            let mut json = String::new();
                            if file.read_to_string(&mut json).is_ok() {
                                if let Ok(params) = serde_json::from_str::<AudioParams>(&json) {
                                    title = params.preset_title;
                                    description = params.preset_description;
                                    experimental = params.experimental;
                                }
                            }
                        }

                        self.preset_list.push(PresetInfo {
                            filename: name,
                            title,
                            description,
                            experimental,
                        });
                    }
                }
            }
        }

        // Add embedded presets that aren't already loaded from user directory
        for preset in embedded_presets::EMBEDDED_PRESETS {
            if !loaded_files.contains(preset.filename) {
                let mut title = None;
                let mut description = None;
                let mut experimental = None;

                if let Ok(params) = serde_json::from_str::<AudioParams>(preset.content) {
                    title = params.preset_title;
                    description = params.preset_description;
                    experimental = params.experimental;
                }

                self.preset_list.push(PresetInfo {
                    filename: preset.filename.to_string(),
                    title,
                    description,
                    experimental,
                });
            }
        }
        // Sort with DEFAULT_ files first
        self.preset_list.sort_by(|a, b| {
            let a_is_default = a.filename.starts_with("DEFAULT_");
            let b_is_default = b.filename.starts_with("DEFAULT_");
            match (a_is_default, b_is_default) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.filename.cmp(&b.filename),
            }
        });
        if !self.preset_list.is_empty() {
             self.preset_state.select(Some(0));
        } else {
             self.preset_state.select(None);
        }
    }
    
    fn enter_preset_mode(&mut self) {
        self.mode = AppMode::PresetSelect;
        self.refresh_presets();
    }
    
    fn exit_preset_mode(&mut self) {
        self.mode = AppMode::Mixer;
    }

    /// Enter pitch-matching: play a pure tone at the current tinnitus pitch so the
    /// user can tune it to match their tinnitus.
    fn enter_pitch_match(&mut self) {
        self.mode = AppMode::PitchMatch;
        self.params.lock().pitch_match_active = true;
    }

    /// Cancel pitch-matching without changing the therapy layer.
    fn cancel_pitch_match(&mut self) {
        self.params.lock().pitch_match_active = false;
        self.mode = AppMode::Mixer;
    }

    /// Lock the matched pitch as the notch center and enable the therapy layer.
    fn confirm_pitch_match(&mut self) {
        let hz = {
            let mut params = self.params.lock();
            params.pitch_match_active = false;
            // Auto-enable the notched-noise layer if it was silent.
            if params.notch_vol <= 0.0 {
                params.notch_vol = 0.5;
            }
            params.tinnitus_freq_hz
        };
        self.mode = AppMode::Mixer;
        self.status_msg = Some((
            format!("Notch centered at {:.0} Hz — notched noise enabled", hz),
            std::time::Instant::now(),
        ));
    }

    /// Adjust the candidate tinnitus pitch during matching (coarse/fine by step size).
    fn adjust_tinnitus_pitch(&mut self, delta: f32) {
        let mut params = self.params.lock();
        let step = if delta.abs() > 0.05 { TINNITUS_FREQ_COARSE_STEP } else { TINNITUS_FREQ_FINE_STEP };
        params.tinnitus_freq_hz = (params.tinnitus_freq_hz + delta.signum() * step)
            .clamp(TINNITUS_FREQ_MIN_HZ, TINNITUS_FREQ_MAX_HZ);
    }
    
    fn load_selected_preset(&mut self) {
        if let Some(i) = self.preset_state.selected() {
            if i < self.preset_list.len() {
                let preset_info = &self.preset_list[i];
                let filename = &preset_info.filename;

                // Use hybrid loading (user dir first, then embedded fallback)
                if let Some(json) = load_preset_hybrid(filename) {
                    if let Ok(mut loaded) = serde_json::from_str::<AudioParams>(&json) {
                        // Safety
                        loaded.rf_enabled = false;

                        // Preserve streaming settings when loading preset
                        let mut params = self.params.lock();
                        let stream_enabled = params.stream_enabled;
                        let stream_port = params.stream_port;
                        let current_version = params.preset_version;

                        loaded.stream_enabled = stream_enabled;
                        loaded.stream_port = stream_port;
                        loaded.preset_version = current_version.wrapping_add(1);

                        *params = loaded;
                        drop(params);

                        // Store preset name
                        self.current_preset = Some(filename.clone());

                        self.status_msg = Some((format!("Loaded {}", filename), std::time::Instant::now()));
                        self.exit_preset_mode();
                        return;
                    }
                }
                self.status_msg = Some(("Failed to load preset".to_string(), std::time::Instant::now()));
            }
        }
    }

    fn save_preset(&mut self) {
        let params = self.params.lock();
        let json = match serde_json::to_string_pretty(&*params) {
            Ok(j) => j,
            Err(e) => {
                self.status_msg = Some((format!("Error serializing: {}", e), std::time::Instant::now()));
                return;
            }
        };

        // Create presets directory if it doesn't exist
        let presets_dir = get_presets_dir();
        if let Err(e) = std::fs::create_dir_all(&presets_dir) {
            self.status_msg = Some((format!("Error creating presets directory: {}", e), std::time::Instant::now()));
            return;
        }

        // Create timestamped filename (never overwrites DEFAULT_ files)
        use std::time::{SystemTime, UNIX_EPOCH};
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let timestamp = since_the_epoch.as_secs();

        let filename = format!("custom_{}.json", timestamp);
        let path = presets_dir.join(filename);

        match File::create(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(json.as_bytes()) {
                     self.status_msg = Some((format!("Error writing file: {}", e), std::time::Instant::now()));
                } else {
                     self.status_msg = Some((format!("Saved to {}", path.display()), std::time::Instant::now()));
                }
            }
            Err(e) => {
                self.status_msg = Some((format!("Error creating file: {}", e), std::time::Instant::now()));
            }
        }
    }

    fn toggle_recording(&mut self, sample_rate: u32) {
        if let Some((path, dropped)) = stop_recording(&self.recording) {
            let msg = if dropped > 0 {
                format!("Recording saved: {} ({} blocks dropped, disk too slow)", path, dropped)
            } else {
                format!("Recording saved: {}", path)
            };
            self.status_msg = Some((msg, std::time::Instant::now()));
        } else {
            // Start recording — create WAV file
            let recordings_dir = get_presets_dir().parent()
                .unwrap_or(std::path::Path::new("."))
                .join("recordings");
            if let Err(e) = std::fs::create_dir_all(&recordings_dir) {
                self.status_msg = Some((
                    format!("Error creating recordings dir: {}", e),
                    std::time::Instant::now(),
                ));
                return;
            }
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            let filename = format!("rec_{}.wav", ts);
            let path = recordings_dir.join(&filename);
            match File::create(&path) {
                Ok(file) => {
                    let mut writer = std::io::BufWriter::new(file);
                    // Write WAV header with placeholder sizes
                    let mut header = Vec::with_capacity(44);
                    header.extend_from_slice(b"RIFF");
                    header.extend_from_slice(&0u32.to_le_bytes()); // placeholder
                    header.extend_from_slice(b"WAVE");
                    header.extend_from_slice(b"fmt ");
                    header.extend_from_slice(&16u32.to_le_bytes());
                    header.extend_from_slice(&WAV_PCM_FORMAT.to_le_bytes());
                    header.extend_from_slice(&WAV_STEREO_CHANNELS.to_le_bytes());
                    header.extend_from_slice(&sample_rate.to_le_bytes());
                    header.extend_from_slice(
                        &(sample_rate * WAV_BLOCK_ALIGN as u32).to_le_bytes(),
                    );
                    header.extend_from_slice(&WAV_BLOCK_ALIGN.to_le_bytes());
                    header.extend_from_slice(&WAV_BITS_PER_SAMPLE.to_le_bytes());
                    header.extend_from_slice(b"data");
                    header.extend_from_slice(&0u32.to_le_bytes()); // placeholder
                    if writer.write_all(&header).is_err() {
                        self.status_msg = Some((
                            "Failed to write WAV header".to_string(),
                            std::time::Instant::now(),
                        ));
                        return;
                    }
                    // All allocation happens here, on the UI thread, never in the callback.
                    let (tx, rx) = std::sync::mpsc::sync_channel::<RecordingMsg>(REC_POOL_BUFFERS);
                    let (pool_tx, pool_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(REC_POOL_BUFFERS);
                    for _ in 0..REC_POOL_BUFFERS {
                        let _ = pool_tx.try_send(Vec::with_capacity(REC_BUFFER_BYTES));
                    }
                    let handle = std::thread::Builder::new()
                        .name("wav-writer".into())
                        .spawn(move || run_recording_writer(writer, rx, pool_tx));
                    let handle = match handle {
                        Ok(h) => h,
                        Err(e) => {
                            self.status_msg = Some((
                                format!("Error starting recording thread: {}", e),
                                std::time::Instant::now(),
                            ));
                            return;
                        }
                    };

                    let mut rec = self.recording.lock();
                    rec.active = true;
                    rec.tx = Some(tx);
                    rec.pool_rx = Some(pool_rx);
                    rec.writer = Some(handle);
                    rec.dropped_blocks.store(0, std::sync::atomic::Ordering::Relaxed);
                    rec.filepath = Some(path.display().to_string());
                    self.status_msg = Some((
                        format!("Recording to {}", filename),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    self.status_msg = Some((
                        format!("Error creating recording: {}", e),
                        std::time::Instant::now(),
                    ));
                }
            }
        }
    }

    fn toggle_collapse(&mut self) {
        if let Some(ui_idx) = self.state.selected() {
            // Map UI index to original channel index
            if ui_idx < self.visible_channel_indices.len() {
                let channel_idx = self.visible_channel_indices[ui_idx];
                if channel_idx < self.channels.len() {
                    let chan = &self.channels[channel_idx];
                    match chan.name.as_str() {
                        "PRESET:" => {
                            // Cycle through: Hidden → Brief → Full → Hidden
                            self.preset_desc_mode = match self.preset_desc_mode {
                                PresetDescMode::Hidden => PresetDescMode::Brief,
                                PresetDescMode::Brief => PresetDescMode::Full,
                                PresetDescMode::Full => PresetDescMode::Hidden,
                            };
                            let state = match self.preset_desc_mode {
                                PresetDescMode::Hidden => "hidden",
                                PresetDescMode::Brief => "brief (3 lines)",
                                PresetDescMode::Full => "full text",
                            };
                            self.status_msg = Some((format!("Preset info: {}", state), std::time::Instant::now()));
                        },
                        "SIGNAL LAYER" => {
                            self.signal_layer_collapsed = !self.signal_layer_collapsed;
                            let state = if self.signal_layer_collapsed { "collapsed" } else { "expanded" };
                            self.status_msg = Some((format!("Signal Layer {}", state), std::time::Instant::now()));
                        },
                        "HACKRF TRANSMIT" => {
                            self.hackrf_collapsed = !self.hackrf_collapsed;
                            let state = if self.hackrf_collapsed { "collapsed" } else { "expanded" };
                            self.status_msg = Some((format!("HackRF Transmit {}", state), std::time::Instant::now()));
                        },
                        name if name.starts_with("NETWORK STREAMING") => {
                            self.streaming_collapsed = !self.streaming_collapsed;
                            let state = if self.streaming_collapsed { "collapsed" } else { "expanded" };
                            self.status_msg = Some((format!("Network Streaming {}", state), std::time::Instant::now()));
                        },
                        name if name.starts_with("TINNITUS THERAPY") => {
                            self.tinnitus_collapsed = !self.tinnitus_collapsed;
                            let state = if self.tinnitus_collapsed { "collapsed" } else { "expanded" };
                            self.status_msg = Some((format!("Tinnitus Therapy {}", state), std::time::Instant::now()));
                        },
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Initialize presets directory and copy embedded presets if needed
fn initialize_presets() -> std::io::Result<()> {
    let presets_dir = get_presets_dir();

    // Create presets directory if it doesn't exist
    std::fs::create_dir_all(&presets_dir)?;

    // Install or refresh the built-in presets in the user directory
    for preset in embedded_presets::EMBEDDED_PRESETS {
        sync_builtin_preset(&presets_dir, preset.filename, preset.content)?;
    }
    // Move aside copies of built-in presets that were renamed
    for filename in embedded_presets::RETIRED_BUILTIN_PRESETS {
        retire_builtin_preset(&presets_dir, filename)?;
    }

    Ok(())
}

/// Unix time in seconds, for backup file names.
fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Move a retired built-in preset aside as `<name>.bak-<unix time>`, which the
/// preset list ignores. Returns true when a file was moved.
fn retire_builtin_preset(dir: &std::path::Path, filename: &str) -> std::io::Result<bool> {
    let path = dir.join(filename);
    if !path.exists() {
        return Ok(false);
    }
    std::fs::rename(&path, dir.join(format!("{}.bak-{}", filename, unix_secs())))?;
    Ok(true)
}

/// Install a built-in (`DEFAULT_`) preset, or refresh it when this version of the
/// app ships different content (corrected text, retuned values). Saving from the UI
/// never writes to `DEFAULT_` files, so a differing copy is an older release's
/// version or a hand edit; either way the old copy is kept next to it as
/// `<name>.bak-<unix time>`, which the preset list ignores (it only lists `.json`).
/// Returns true when a file was written.
fn sync_builtin_preset(dir: &std::path::Path, filename: &str, content: &str) -> std::io::Result<bool> {
    let path = dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(existing) if existing == content => Ok(false),
        Ok(_) => {
            std::fs::rename(&path, dir.join(format!("{}.bak-{}", filename, unix_secs())))?;
            std::fs::write(&path, content)?;
            Ok(true)
        }
        Err(_) => {
            std::fs::write(&path, content)?;
            Ok(true)
        }
    }
}

/// Load preset from user directory or embedded fallback
fn load_preset_hybrid(filename: &str) -> Option<String> {
    // Try user directory first
    let user_path = get_presets_dir().join(filename);
    if user_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&user_path) {
            return Some(content);
        }
    }

    // Fallback to embedded preset
    for preset in embedded_presets::EMBEDDED_PRESETS {
        if preset.filename == filename {
            return Some(preset.content.to_string());
        }
    }

    None
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize presets directory and copy embedded presets on first run
    let _ = initialize_presets();

    // 1. Audio Setup
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");
    let supported_config = device.default_output_config()?;
    let sample_rate = supported_config.sample_rate() as f32;

    // Request explicit buffer size (env override for Bluetooth/troubleshooting)
    let buffer_frames = std::env::var("SOULWHISTLE_BUFFER_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_BUFFER_FRAMES);
    let config = {
        let mut cfg: cpal::StreamConfig = supported_config.clone().into();
        cfg.buffer_size = cpal::BufferSize::Fixed(buffer_frames);
        // Verify the device accepts this — rebuild with default if not
        match device.build_output_stream(
            &cfg,
            |_data: &mut [f32], _: &cpal::OutputCallbackInfo| {},
            |_| {},
            None,
        ) {
            Ok(_) => (), // Device accepts our buffer size
            Err(_) => {
                // Fall back to device default
                cfg.buffer_size = cpal::BufferSize::Default;
            }
        }
        cfg
    };

    let mut initial_params = AudioParams::default();
    let mut loaded_preset_name: Option<String> = None;

    // Try to load preset.json first (from current directory for backwards compatibility)
    let preset_loaded = if let Ok(mut file) = File::open(PRESET_FILENAME) {
        use std::io::Read;
        let mut json = String::new();
        if file.read_to_string(&mut json).is_ok() {
             if let Ok(loaded) = serde_json::from_str::<AudioParams>(&json) {
                 initial_params = loaded;
                 initial_params.rf_enabled = false; // Safety
                 loaded_preset_name = Some(PRESET_FILENAME.to_string());
                 true
             } else {
                 false
             }
        } else {
            false
        }
    } else {
        false
    };

    // If no preset.json, load default deep focus preset using hybrid loading
    if !preset_loaded {
        if let Some(json) = load_preset_hybrid(DEFAULT_PRESET_FILENAME) {
            if let Ok(loaded) = serde_json::from_str::<AudioParams>(&json) {
                initial_params = loaded;
                initial_params.rf_enabled = false; // Safety
                loaded_preset_name = Some(DEFAULT_PRESET_FILENAME.to_string());
            }
        }
    }

    let params = Arc::new(Mutex::new(initial_params));
    let audio_params = params.clone();
    let rf_params = params.clone();
    let stream_params = params.clone();
    let (error_tx, error_rx) = std::sync::mpsc::channel::<String>();

    // Create recording state (shared between audio callback and UI)
    let recording_state = Arc::new(Mutex::new(RecordingState::new()));
    let recording_for_audio = recording_state.clone();

    // Create streaming buffer
    let stream_buffer = Arc::new(AudioRingBuffer::new(sample_rate as u32, STREAM_BUFFER_DURATION_MS));
    let stream_buffer_for_audio = stream_buffer.clone();
    let stream_buffer_for_server = stream_buffer.clone();

    // Start RF Thread (pass the actual audio sample rate)
    let rf_sample_rate = sample_rate;
    std::thread::spawn(move || {
        let mut rf = RfWorker::new(rf_params, error_tx, rf_sample_rate);
        rf.run();
    });

    // Start Streaming Server Thread
    let stream_client_count = Arc::new(Mutex::new(0));
    let stream_client_count_for_app = stream_client_count.clone();
    let stream_client_count_for_server = stream_client_count.clone();
    
    std::thread::spawn(move || {
        loop {
            // Check if streaming is enabled
            let (enabled, port) = {
                let p = stream_params.lock();
                (p.stream_enabled, p.stream_port)
            };
            
            if enabled {
                *stream_client_count_for_server.lock() = 0; // Reset count
                let server = StreamingServer::new(
                    stream_buffer_for_server.clone(),
                    port,
                    stream_client_count_for_server.clone(),
                    stream_params.clone(),
                );
                server.run(); // Returns when streaming is disabled or the port changes
            }
            
            // If not enabled or server stopped, wait and check again
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    // Run audio in a separate thread (handled by cpal stream)

    let stream = match supported_config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config, audio_params, stream_buffer_for_audio, recording_for_audio.clone(), sample_rate),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config, audio_params, stream_buffer_for_audio, recording_for_audio.clone(), sample_rate),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config, audio_params, stream_buffer_for_audio, recording_for_audio, sample_rate),
        _ => panic!("Unsupported sample format"),
    }?;

    stream.play()?;

    // 2. TUI Setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let recording_for_exit = recording_state.clone();
    let mut app = App::new(params, stream_client_count_for_app, recording_state);
    app.current_preset = loaded_preset_name;
    app.sample_rate = sample_rate;
    app.refresh_presets();
    
    let res = run_app(&mut terminal, app, error_rx, sample_rate as u32);

    // Complete an in-progress recording before exiting, or the WAV is left with
    // zero sizes in its header and the last buffered audio is lost.
    let saved_recording = stop_recording(&recording_for_exit);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Some((path, dropped)) = saved_recording {
        println!("Recording saved: {}", path);
        if dropped > 0 {
            println!("Warning: {} blocks dropped (disk too slow)", dropped);
        }
    }

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    params: Arc<Mutex<AudioParams>>,
    stream_buffer: Arc<AudioRingBuffer>,
    recording: Arc<Mutex<RecordingState>>,
    sample_rate: f32,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::Sample + cpal::FromSample<f32> + cpal::SizedSample,
{
    let mut synth = Synthesizer::new(sample_rate);
    let channels = config.channels as usize;

    // Preset change tracking
    let mut last_preset_version: u32 = 0;
    let mut crossfade_samples_remaining: usize = 0;
    let crossfade_duration_samples = (sample_rate * 0.05) as usize; // 50ms crossfade

    // Cache last params to use when lock contention occurs (realtime audio best practice)
    let mut cached_params = AudioParams::default();

    // Pre-allocate streaming sample buffer — reused each callback via clear()
    // Avoids heap allocation in the audio callback hot path
    // Sized above any normal callback (BufferSize::Default can exceed 1024 frames).
    let mut stream_samples: Vec<(f32, f32)> = Vec::with_capacity(8192);

    // Set on preset change; cleared once the streaming buffer flush succeeds.
    let mut flush_pending = false;

    // A recording buffer taken from the pool but not yet handed to the writer
    // (kept when the writer queue was full, reused on the next callback).
    let mut rec_spare: Option<Vec<u8>> = None;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // Use try_lock to avoid blocking if UI thread holds the lock
            // If lock fails, use cached params from last successful lock
            if let Some(guard) = params.try_lock() {
                cached_params = guard.clone_for_audio();
            }
            // Always work from cached_params (avoids a second clone)
            let p = &cached_params;

            // Detect preset change
            if p.preset_version != last_preset_version {
                // Preset changed - reset synthesizer state and flush streaming buffer
                synth.reset();
                flush_pending = true;

                // Start crossfade
                crossfade_samples_remaining = crossfade_duration_samples;
                last_preset_version = p.preset_version;
            }

            // Non-blocking flush; retried each callback until the reader lets go.
            if flush_pending && stream_buffer.try_flush() {
                flush_pending = false;
            }

            // Reuse pre-allocated buffer (clear keeps capacity, zero allocs)
            stream_samples.clear();

            // Check recording state once per callback (avoid per-sample lock).
            // Under a try_lock we only clone the sender and the drop counter
            // (refcount bumps) and take a free pooled buffer: no allocation, no I/O.
            let rec_session = recording.try_lock().and_then(|r| {
                if !r.active {
                    return None;
                }
                let tx = r.tx.clone()?;
                let buf = rec_spare.take()
                    .or_else(|| r.pool_rx.as_ref().and_then(|p| p.try_recv().ok()));
                Some((tx, buf, r.dropped_blocks.clone()))
            });
            let collect_samples = p.stream_enabled || rec_session.is_some();

            for frame in data.chunks_mut(channels) {
                let (mut left_sample_f32, mut right_sample_f32) = synth.next_sample(p);

                // Apply equal-power crossfade if active
                if crossfade_samples_remaining > 0 {
                    let progress = 1.0 - (crossfade_samples_remaining as f32 / crossfade_duration_samples as f32);
                    // Equal-power curve: sin(progress * π/2) — maintains constant perceived loudness
                    let fade_multiplier = (progress * std::f32::consts::FRAC_PI_2).sin();
                    left_sample_f32 *= fade_multiplier;
                    right_sample_f32 *= fade_multiplier;
                    crossfade_samples_remaining -= 1;
                }

                // Collect for batch streaming and/or recording (avoid per-sample lock)
                if collect_samples {
                    stream_samples.push((left_sample_f32, right_sample_f32));
                }

                if channels >= 2 {
                    // Output stereo
                    frame[0] = T::from_sample(left_sample_f32);
                    frame[1] = T::from_sample(right_sample_f32);
                } else {
                    // Fallback to mono for single-channel devices
                    let mono_sample = (left_sample_f32 + right_sample_f32) * 0.5;
                    frame[0] = T::from_sample(mono_sample);
                }
            }

            // Batch push to streaming buffer (single lock acquisition)
            if !stream_samples.is_empty() {
                stream_buffer.push_samples_batch(&stream_samples);
            }

            // Hand recording bytes to the writer thread: a non-blocking try_send of a
            // pooled buffer. Disk I/O happens on the writer thread.
            if let Some((tx, buf, dropped)) = rec_session {
                if !stream_samples.is_empty() {
                    match buf {
                        Some(mut buf) => {
                            buf.clear();
                            for &(l, r) in &stream_samples {
                                let li = (l.clamp(-1.0, 1.0) * PCM_I16_MAX) as i16;
                                let ri = (r.clamp(-1.0, 1.0) * PCM_I16_MAX) as i16;
                                buf.extend_from_slice(&li.to_le_bytes());
                                buf.extend_from_slice(&ri.to_le_bytes());
                            }
                            if let Err(std::sync::mpsc::TrySendError::Full(RecordingMsg::Data(b))) =
                                tx.try_send(RecordingMsg::Data(buf))
                            {
                                // Writer queue full: keep the buffer for next time.
                                rec_spare = Some(b);
                                dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        // No free buffer: the writer is behind the disk.
                        None => {
                            dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }

            // Update session info using try_lock to avoid blocking audio
            let (session_timer, session_phase) = synth.coherence.get_session_info();
            if let Some(mut params_write) = params.try_lock() {
                params_write.session_timer = session_timer;
                params_write.session_phase = session_phase;
            }
            // If lock fails, skip update this callback - UI will get updated next time
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App, error_rx: std::sync::mpsc::Receiver<String>, sample_rate: u32) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Check for RF worker errors
        if let Ok(err_msg) = error_rx.try_recv() {
            app.status_msg = Some((err_msg, std::time::Instant::now()));
        }

        if event::poll(std::time::Duration::from_millis(EVENT_POLL_INTERVAL_MS))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        AppMode::Mixer => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('s') => app.save_preset(),
                                KeyCode::Char('r') => app.toggle_recording(sample_rate),
                                KeyCode::Char('l') => app.enter_preset_mode(),
                                KeyCode::Char('o') => app.cycle_modulation(),
                                KeyCode::Char('m') => app.toggle_mute(),
                                KeyCode::Char(' ') => app.toggle_playback(),
                                KeyCode::Char('x') => app.toggle_collapse(),
                                KeyCode::Char('t') => app.enter_pitch_match(),
                                KeyCode::Down | KeyCode::Char('j') => app.next(),
                                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                                KeyCode::Left | KeyCode::Char('h') => app.adjust_volume(-0.01),
                                KeyCode::Right => app.adjust_volume(0.01),
                                _ => {}
                            }
                        },
                        AppMode::PitchMatch => {
                            match key.code {
                                KeyCode::Enter => app.confirm_pitch_match(),
                                KeyCode::Esc | KeyCode::Char('q') => app.cancel_pitch_match(),
                                // Coarse adjust on Up/Down, fine on Left/Right
                                KeyCode::Up | KeyCode::Char('k') => app.adjust_tinnitus_pitch(1.0),
                                KeyCode::Down | KeyCode::Char('j') => app.adjust_tinnitus_pitch(-1.0),
                                KeyCode::Right | KeyCode::Char('l') => app.adjust_tinnitus_pitch(0.01),
                                KeyCode::Left | KeyCode::Char('h') => app.adjust_tinnitus_pitch(-0.01),
                                _ => {}
                            }
                        },
                        AppMode::PresetSelect => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => app.exit_preset_mode(),
                                KeyCode::Enter => app.load_selected_preset(),
                                KeyCode::Down | KeyCode::Char('j') => app.next(),
                                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(4)].as_ref())
        .split(f.area());

    match app.mode {
        AppMode::Mixer => draw_mixer(f, app, chunks[0]),
        AppMode::PresetSelect => draw_preset_list(f, app, chunks[0]),
        AppMode::PitchMatch => draw_pitch_match(f, app, chunks[0]),
    }

    // Build compact 2-line status display
    let params = app.params.lock();
    
    // Line 1: Playback state | Master Vol | Being/Mode | RF Status
    let playback_icon = if params.playing { "▶" } else { "⏸" };
    let being_icon = match params.coherence.being_type {
        crate::coherence::BeingType::Unknown => "🛸",
        crate::coherence::BeingType::HumanFocus10 => "🧠",
        crate::coherence::BeingType::HumanFocus12 => "🌌",
        crate::coherence::BeingType::HumanFocus15 => "⏱️",
        crate::coherence::BeingType::HumanFocus21 => "🌉",
        crate::coherence::BeingType::HumanCustom => "⚙️",
    };
    // Show preset title if available, otherwise fall back to being type
    let being_short = if let Some(ref title) = params.preset_title {
        // Show preset title with frequency
        match params.coherence.being_type {
            crate::coherence::BeingType::Unknown => title.clone(),
            crate::coherence::BeingType::HumanCustom => {
                format!("{} ({:.1}Hz)", title, params.coherence.custom_binaural_hz)
            },
            _ => {
                format!("{} ({:.1}Hz)", title, params.coherence.binaural_beat_hz())
            }
        }
    } else {
        // Fall back to being type name
        match params.coherence.being_type {
            crate::coherence::BeingType::Unknown => {
                if let Some(ref preset_name) = app.current_preset {
                    format!("signal-layer ({})", preset_name)
                } else {
                    "signal-layer".to_string()
                }
            },
            crate::coherence::BeingType::HumanFocus10 => format!("human-focus10 ({:.1}Hz)", params.coherence.binaural_beat_hz()),
            crate::coherence::BeingType::HumanFocus12 => format!("human-focus12 ({:.1}Hz)", params.coherence.binaural_beat_hz()),
            crate::coherence::BeingType::HumanFocus15 => format!("human-focus15 ({:.1}Hz)", params.coherence.binaural_beat_hz()),
            crate::coherence::BeingType::HumanFocus21 => format!("human-focus21 ({:.1}Hz OBE)", params.coherence.binaural_beat_hz()),
            crate::coherence::BeingType::HumanCustom => {
                if let Some(ref preset_name) = app.current_preset {
                    format!("human-custom ({:.1}Hz) ({})", params.coherence.custom_binaural_hz, preset_name)
                } else {
                    format!("human-custom ({:.1}Hz)", params.coherence.custom_binaural_hz)
                }
            },
        }
    };
    let rf_status = if params.rf_enabled {
        format!("RF: ON {:.1}MHz", params.rf_freq_hz as f64 / 1_000_000.0)
    } else {
        "RF: OFF".to_string()
    };
    
    let is_recording = app.recording.lock().active;
    // Blink: 500ms on / 500ms off
    let blink_on = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() % 1000 < 500;

    let base_info = format!(
        "{} {} | Master: {:.0}% | {} {} | {}",
        playback_icon,
        if params.playing { "Playing" } else { "Paused" },
        params.master_vol * 100.0,
        being_icon,
        being_short,
        rf_status,
    );

    // Build line1 with styled spans (blinking red REC dot)
    let mut line1_spans = vec![Span::raw(base_info)];
    if is_recording {
        if blink_on {
            line1_spans.push(Span::styled(
                " ● REC",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        } else {
            line1_spans.push(Span::styled(
                "   REC",
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // Line 2: Keybindings | Status message
    let mut line2_str = "[m]ute [o]scillator [t]innitus [r]ecord [Space]pause [q]uit".to_string();

    if let Some((text, time)) = &app.status_msg {
        // Show warnings (⚠️) for longer
        let timeout = if text.starts_with("⚠️") { STATUS_WARNING_TIMEOUT_SECS } else { STATUS_TIMEOUT_SECS };
        if time.elapsed() < std::time::Duration::from_secs(timeout) {
            line2_str = format!("[m]ute [o]scillator [t]innitus [r]ecord [Space]pause [q]uit | STATUS: {}", text);
        }
    }

    drop(params); // Release lock

    let status_lines = vec![
        Line::from(line1_spans),
        Line::from(line2_str),
    ];
    let instructions = Paragraph::new(status_lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(instructions, chunks[1]);
}

fn draw_pitch_match(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let hz = app.params.lock().tinnitus_freq_hz;
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  TINNITUS PITCH MATCHING",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  A pure tone is playing. Tune it until it matches the pitch of"),
        Line::from("  your tinnitus, then press Enter to center the notch there."),
        Line::from("  🎧 Use headphones at a comfortable, low volume."),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Current pitch:  "),
            Span::styled(
                format!("{:.0} Hz", hz),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ↑/↓  coarse (±250 Hz)    ←/→  fine (±25 Hz)",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "  Enter  lock pitch & enable notch     Esc  cancel",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Note: notched therapy works best for tonal tinnitus ≤ 8 kHz,",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  used daily over weeks. It is an adjunct, not a quick fix.",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let panel = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Pitch Match "));
    f.render_widget(panel, area);
}

fn draw_preset_list(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Split area into list and detail sections - give more space to details
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(area);

    let items: Vec<ListItem> = app.preset_list.iter()
        .map(|preset_info| {
            let experimental_tag = if preset_info.experimental == Some(true) {
                "[EXPERIMENTAL] "
            } else {
                ""
            };
            let display_name = if preset_info.filename.starts_with("DEFAULT_") {
                if let Some(ref title) = preset_info.title {
                    format!("🔒 {}{}", experimental_tag, title)
                } else {
                    format!("🔒 {}{}", experimental_tag, preset_info.filename)
                }
            } else {
                if let Some(ref title) = preset_info.title {
                    format!("   {}{}", experimental_tag, title)
                } else {
                    format!("   {}{}", experimental_tag, preset_info.filename)
                }
            };
            ListItem::new(Line::from(display_name))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Available Presets (↑↓: Navigate, Enter: Load, Esc: Cancel)"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], &mut app.preset_state);

    // Show details for selected preset
    if let Some(i) = app.preset_state.selected() {
        if i < app.preset_list.len() {
            let preset_info = &app.preset_list[i];
            let mut detail_text = String::new();

            // Title section
            if let Some(ref title) = preset_info.title {
                detail_text.push_str(&format!("━━ {} ━━\n\n", title));
            } else {
                detail_text.push_str(&format!("━━ {} ━━\n\n", preset_info.filename));
            }

            // Description section
            if let Some(ref desc) = preset_info.description {
                let max_width = chunks[1].width.saturating_sub(4) as usize;
                let wrapped = textwrap::fill(desc, max_width);
                detail_text.push_str(&wrapped);
                detail_text.push_str("\n\n");
            }

            // File info
            detail_text.push_str(&format!("File: {}", preset_info.filename));

            let detail = Paragraph::new(detail_text)
                .block(Block::default().borders(Borders::ALL).title("Preset Information"))
                .style(Style::default().fg(Color::White));
            f.render_widget(detail, chunks[1]);
        }
    }
}

fn draw_mixer(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Split: fixed header (logo + ON AIR) and scrollable channel list
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(area);
    let header_area = layout[0];
    let list_area = layout[1];

    // Get current params to display
    let params = app.params.lock();

    // ON AIR retro indicator (blinks when playing)
    let blink_on = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() % 1000 < 600;
    let on_air_style = if params.playing && blink_on {
        Style::default().fg(Color::Red).bg(Color::Black).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray).bg(Color::Black)
    };

    // Render fixed header as Paragraph (non-selectable, non-scrollable)
    let header_lines = vec![
        Line::from(" ███████╗ ██████╗ ██╗   ██╗██╗     ██╗    ██╗██╗  ██╗██╗███████╗████████╗██╗     ███████╗"),
        Line::from(" ██╔════╝██╔═══██╗██║   ██║██║     ██║    ██║██║  ██║██║██╔════╝╚══██╔══╝██║     ██╔════╝"),
        Line::from(" ███████╗██║   ██║██║   ██║██║     ██║ █╗ ██║███████║██║███████╗   ██║   ██║     █████╗  "),
        Line::from(" ╚════██║██║   ██║██║   ██║██║     ██║███╗██║██╔══██║██║╚════██║   ██║   ██║     ██╔══╝  "),
        Line::from(" ███████║╚██████╔╝╚██████╔╝███████╗╚███╔███╔╝██║  ██║██║███████║   ██║   ███████╗███████╗"),
        Line::from(" ╚══════╝ ╚═════╝  ╚═════╝ ╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝╚══════╝"),
        Line::from(""),
        Line::from(vec![
            Span::raw("         "),
            Span::styled(" ■ ON AIR ■ ", on_air_style),
            Span::raw("  Multi-Dimensional Communication System"),
        ]),
    ];
    let header = Paragraph::new(header_lines)
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));
    f.render_widget(header, header_area);

    // Build visible channel indices and items (no spacers, no header offset)
    let items_with_indices: Vec<(usize, ListItem)> = app
        .channels
        .iter()
        .enumerate()
        .filter_map(|(idx, chan)| {
            // Skip collapsed section children
            if chan.name.starts_with("  ") {
                for i in (0..idx).rev() {
                    let parent = &app.channels[i];
                    if !parent.name.starts_with("  ") {
                        if parent.name == "SIGNAL LAYER" && app.signal_layer_collapsed {
                            return None;
                        }
                        if parent.name == "HACKRF TRANSMIT" && app.hackrf_collapsed {
                            return None;
                        }
                        if parent.name.starts_with("NETWORK STREAMING") && app.streaming_collapsed {
                            return None;
                        }
                        if parent.name.starts_with("TINNITUS THERAPY") && app.tinnitus_collapsed {
                            return None;
                        }
                        break;
                    }
                }
            }

            // Section headers with special rendering
            if chan.name == "PRESET:" {
                let preset_name = if let Some(ref title) = params.preset_title {
                    title.clone()
                } else {
                    match params.coherence.being_type {
                        crate::coherence::BeingType::Unknown => "signal-layer".to_string(),
                        crate::coherence::BeingType::HumanFocus10 => "human-focus10".to_string(),
                        crate::coherence::BeingType::HumanFocus12 => "human-focus12".to_string(),
                        crate::coherence::BeingType::HumanFocus15 => "human-focus15".to_string(),
                        crate::coherence::BeingType::HumanFocus21 => "human-focus21".to_string(),
                        crate::coherence::BeingType::HumanCustom => "human-custom".to_string(),
                    }
                };
                let experimental_tag = if params.experimental == Some(true) { "[EXPERIMENTAL] " } else { "" };
                let indicator = match app.preset_desc_mode {
                    PresetDescMode::Hidden => "[+]",
                    PresetDescMode::Brief => "[-]",
                    PresetDescMode::Full => "[=]",
                };
                let section_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
                return Some((idx, ListItem::new(Line::from(vec![
                    Span::styled("── ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("PRESET: {}{} {}", experimental_tag, preset_name, indicator),
                        section_style,
                    ),
                    Span::styled("  [s]ave [l]oad [x]info", Style::default().fg(Color::DarkGray)),
                ]))));
            }

            if chan.name == "PRESET_DESC" {
                match app.preset_desc_mode {
                    PresetDescMode::Hidden => return None,
                    PresetDescMode::Brief => {
                        if let Some(ref description) = params.preset_description {
                            let mut text_lines = wrap_text(description, DESC_MAX_WIDTH, Some(DESC_BRIEF_LINES), "  ");
                            if description.len() > DESC_BRIEF_CHAR_THRESHOLD {
                                text_lines.push("  ... (press [x] on PRESET line for full text)".to_string());
                            }
                            return Some((idx, ListItem::new(text_lines.join("\n"))));
                        }
                        return None;
                    },
                    PresetDescMode::Full => {
                        if let Some(ref description) = params.preset_description {
                            let text_lines = wrap_text(description, DESC_MAX_WIDTH, None, "  ");
                            return Some((idx, ListItem::new(text_lines.join("\n"))));
                        }
                        return None;
                    }
                }
            }

            if chan.name == "BINAURAL BEATS" {
                let headphones_note = if !matches!(params.coherence.being_type, crate::coherence::BeingType::Unknown) {
                    "     [🎧 HEADPHONES REQUIRED]"
                } else {
                    ""
                };
                let section_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
                return Some((idx, ListItem::new(Line::from(vec![
                    Span::styled("── ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("BINAURAL BEATS{}", headphones_note), section_style),
                ]))));
            }

            // Collapsible section headers
            if chan.name == "SIGNAL LAYER" || chan.name == "HACKRF TRANSMIT"
                || chan.name.starts_with("NETWORK STREAMING")
                || chan.name.starts_with("TINNITUS THERAPY")
            {
                let (collapsed, item_count) = match chan.name.as_str() {
                    "SIGNAL LAYER" => (app.signal_layer_collapsed, 7),
                    "HACKRF TRANSMIT" => (app.hackrf_collapsed, 5),
                    name if name.starts_with("TINNITUS THERAPY") => (app.tinnitus_collapsed, 4),
                    _ => (app.streaming_collapsed, 2),
                };
                let indicator = if collapsed { "[+]" } else { "[-]" };
                let status = if collapsed {
                    format!("{} items hidden", item_count)
                } else {
                    "expanded".to_string()
                };
                let section_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
                return Some((idx, ListItem::new(Line::from(vec![
                    Span::styled("── ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} {}", chan.name, indicator), section_style),
                    Span::styled(format!(" ({})  [x]toggle", status), Style::default().fg(Color::DarkGray)),
                ]))));
            }

            // Regular channel items
            let content = match chan.id {
                ChannelId::BeingType => String::new(),
                ChannelId::CoherenceVol => {
                    if matches!(params.coherence.being_type, crate::coherence::BeingType::Unknown) {
                        format!("{:<40} [Disabled - select a preset]", chan.name)
                    } else {
                        let filled = (params.coherence.volume * 20.0) as usize;
                        let bar: String = std::iter::repeat("█").take(filled).collect();
                        let empty: String = std::iter::repeat("░").take(20 - filled).collect();
                        let beat_hz = params.coherence.binaural_beat_hz();
                        let state = params.coherence.brainwave_state();
                        format!("{:<40} [{}{}] {:.0}% {:.1}Hz {}",
                            chan.name, bar, empty, params.coherence.volume * 100.0, beat_hz, state)
                    }
                },
                ChannelId::PingFreq => {
                    // The synth mutes the ping above this limit (audio.rs), so say so
                    // instead of letting an ultrasonic preset play silence unexplained.
                    let limit = app.sample_rate * NYQUIST_SAFE_FRACTION;
                    if app.sample_rate > 0.0 && params.ping_freq_hz > limit {
                        format!("{:<40} {:.2}kHz MUTED: above {:.1}kHz limit at {:.1}kHz output (needs a higher sample rate)",
                            chan.name, params.ping_freq_hz / 1000.0, limit / 1000.0, app.sample_rate / 1000.0)
                    } else {
                        format!("{:<40} {:.2}kHz (arrows to adjust)",
                            chan.name, params.ping_freq_hz / 1000.0)
                    }
                },
                ChannelId::TinnitusPitch => {
                    format!("{:<40} {:.0} Hz  [t] match tinnitus pitch",
                        chan.name, params.tinnitus_freq_hz)
                },
                ChannelId::NotchWidth => {
                    format!("{:<40} {:.2} oct (arrows to adjust)",
                        chan.name, params.notch_width_oct)
                },
                ChannelId::NotchSource => {
                    let src = if params.notch_use_pink { "Pink noise" } else { "White noise" };
                    format!("{:<40} {} (press o)", chan.name, src)
                },
                ChannelId::BinauralAdjust => {
                    if matches!(params.coherence.being_type, BeingType::HumanCustom) {
                        format!("{:<40} {:.2}Hz (arrows to adjust)",
                            chan.name, params.coherence.custom_binaural_hz)
                    } else {
                        format!("{:<40} [Locked - switch to custom mode]", chan.name)
                    }
                },
                ChannelId::SessionTimer => {
                    let minutes = (params.session_timer / 60.0) as u32;
                    let seconds = (params.session_timer % 60.0) as u32;
                    let phase_name = match params.session_phase {
                        crate::coherence::SessionPhase::Startup => "Startup",
                        crate::coherence::SessionPhase::Induction => "Induction",
                        crate::coherence::SessionPhase::Stabilization => "Stabilization",
                        crate::coherence::SessionPhase::Return => "Return",
                    };
                    let total_min = params.coherence.total_session_min() as u32;
                    let recommendation = if minutes < 15 {
                        format!(" ({} min session)", total_min)
                    } else if minutes >= total_min {
                        " (session complete)".to_string()
                    } else {
                        "".to_string()
                    };
                    format!("{:<40} {:02}:{:02} - {}{}",
                        chan.name, minutes, seconds, phase_name, recommendation)
                },
                ChannelId::RfEnable => {
                    let detection = if params.rf_detected { "✓" } else { "✗" };
                    let state = if params.rf_enabled { "ON " } else { "OFF" };
                    let warning = if params.rf_enabled && !params.rf_detected { " NO DEVICE!" } else { "" };
                    format!("{:<40} {} {} {}", chan.name, state, detection, warning)
                },
                ChannelId::RfFreq => {
                    format!("{:<40} {:.1}MHz {:?}", chan.name, params.rf_freq_hz as f64 / 1_000_000.0, params.rf_mode)
                },
                ChannelId::RfGain => {
                    format!("{:<40} {}dB", chan.name, params.rf_gain)
                },
                ChannelId::RfMode => {
                    format!("{:<40} {:?} (press o)", chan.name, params.rf_mode)
                },
                ChannelId::RfPulseType => {
                    format!("{:<40} {:?} (press o)", chan.name, params.rf_pulse_type)
                },
                ChannelId::StreamEnable => {
                    let detection = if params.stream_enabled { "✓" } else { "✗" };
                    let state = if params.stream_enabled { "ON " } else { "OFF" };
                    let client_count = *app.stream_client_count.lock();
                    let clients_info = if params.stream_enabled && client_count > 0 {
                        format!(" ({} client{})", client_count, if client_count == 1 { "" } else { "s" })
                    } else {
                        String::new()
                    };
                    format!("{:<40} {} {}{}", chan.name, state, detection, clients_info)
                },
                ChannelId::StreamPort => {
                    format!("{:<40} {} (http://<ip>:{}/stream.wav)",
                        chan.name, params.stream_port, params.stream_port)
                },
                _ => {
                    let (vol, mod_type) = match chan.id {
                        ChannelId::Master => (params.master_vol, None),
                        ChannelId::Carrier => (params.carrier_vol, Some(params.carrier_type)),
                        ChannelId::Harmonic => (params.harmonic_vol, Some(params.harmonic_type)),
                        ChannelId::Ping => (params.ping_vol, Some(params.ping_type)),
                        ChannelId::Chirp => (params.chirp_vol, Some(params.chirp_type)),
                        ChannelId::Pad => (params.pad_vol, Some(params.pad_type)),
                        ChannelId::Breath => (params.breath_vol, Some(params.breath_type)),
                        ChannelId::NotchVol => (params.notch_vol, None),
                        _ => (0.0, None),
                    };
                    let is_signal_channel = matches!(chan.id,
                        ChannelId::Carrier | ChannelId::Harmonic | ChannelId::Ping |
                        ChannelId::Chirp | ChannelId::Pad | ChannelId::Breath);
                    let filled = (vol * 20.0) as usize;
                    let bar: String = std::iter::repeat("█").take(filled).collect();
                    let empty: String = std::iter::repeat("░").take(20 - filled).collect();
                    let suffix = if is_signal_channel && params.lock_signal_layer { " [LOCKED]" } else { "" };
                    if let Some(mt) = mod_type {
                        format!("{:<40} [{}{}] {:.0}% {:?}{}", chan.name, bar, empty, vol * 100.0, mt, suffix)
                    } else {
                        format!("{:<40} [{}{}] {:.0}%{}", chan.name, bar, empty, vol * 100.0, suffix)
                    }
                }
            };
            Some((idx, ListItem::new(Line::from(content))))
        })
        .collect();

    // Split into indices and items
    let (visible_indices, items): (Vec<usize>, Vec<ListItem>) = items_with_indices.into_iter().unzip();

    // Store the mapping in app
    drop(params);
    app.visible_channel_indices = visible_indices;

    // Render channel list directly — no header offset needed
    let list = List::new(items)
        .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, list_area, &mut app.state);
}

#[cfg(test)]
mod preset_content_tests {
    use super::*;
    use crate::coherence::BeingType;

    /// A tone added to both ears close to a binaural carrier beats against it inside
    /// each ear (a physical, monaural beat), which swamps the intended binaural beat.
    /// explore_all once had a 220 Hz harmonic over 200/207.5 Hz carriers: real 20 and
    /// 12.5 Hz beats on top of the labelled 7.5 Hz one.
    #[test]
    fn harmonics_stay_clear_of_carriers() {
        const MIN_SEPARATION_HZ: f32 = 50.0;
        for preset in embedded_presets::EMBEDDED_PRESETS {
            let params: AudioParams = serde_json::from_str(preset.content)
                .unwrap_or_else(|e| panic!("{}: {e}", preset.filename));
            let c = &params.coherence;
            if !c.enabled {
                continue;
            }
            let mut carriers = vec![c.left_carrier, c.right_carrier];
            if c.being_type == BeingType::HumanFocus10 {
                carriers.extend([GAMMA_CARRIER_HZ, GAMMA_CARRIER_HZ + GAMMA_BEAT_HZ]);
            }
            // Every other tone played into both ears: the coherence harmonics, and
            // the signal layer's tonal channels when the layer is audible.
            let signal_on = !params.lock_signal_layer;
            let carrier_layer = match params.carrier_type {
                SignalType::Schumann783AM => CARRIER_783_HZ,
                _ => CARRIER_BASE_HZ,
            };
            let harmonics = [
                (c.harmonic_220hz, HARMONIC_220_HZ),
                (c.harmonic_495hz, HARMONIC_495_HZ),
                (signal_on && params.harmonic_vol > 0.0, SOLFEGGIO_MI_HZ),
                (signal_on && params.pad_vol > 0.0, VERDI_A_HZ),
                (signal_on && params.carrier_vol > 0.0, carrier_layer),
            ];
            for (on, h) in harmonics {
                if !on {
                    continue;
                }
                for &carrier in &carriers {
                    assert!((h - carrier).abs() >= MIN_SEPARATION_HZ,
                        "{}: {h} Hz tone is only {:.1} Hz from the {carrier} Hz carrier",
                        preset.filename, (h - carrier).abs());
                }
            }
        }
    }
}

#[cfg(test)]
mod preset_sync_tests {
    use super::*;

    #[test]
    fn builtin_presets_are_installed_refreshed_and_backed_up() {
        let dir = std::env::temp_dir().join(format!("soulwhistle-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let name = "DEFAULT_test.json";

        // Fresh install.
        assert!(sync_builtin_preset(&dir, name, "v1").unwrap());
        // Same content: untouched.
        assert!(!sync_builtin_preset(&dir, name, "v1").unwrap());
        // New release: refreshed, old copy kept as a non-.json backup.
        assert!(sync_builtin_preset(&dir, name, "v2").unwrap());
        assert_eq!(std::fs::read_to_string(dir.join(name)).unwrap(), "v2");
        let backups: Vec<String> = std::fs::read_dir(&dir).unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with("DEFAULT_test.json.bak-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read_to_string(dir.join(&backups[0])).unwrap(), "v1");
        assert!(!backups[0].ends_with(".json"));

        // Retired name: moved aside, and a second run does nothing.
        let old = "DEFAULT_old.json";
        std::fs::write(dir.join(old), "old").unwrap();
        assert!(retire_builtin_preset(&dir, old).unwrap());
        assert!(!dir.join(old).exists());
        assert!(!retire_builtin_preset(&dir, old).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn retired_names_are_not_shipped_again() {
        for retired in embedded_presets::RETIRED_BUILTIN_PRESETS {
            assert!(embedded_presets::EMBEDDED_PRESETS.iter().all(|p| p.filename != *retired),
                "{retired} is both retired and shipped");
        }
    }
}

#[cfg(test)]
mod recording_tests {
    use super::*;

    #[test]
    fn writer_patches_wav_header_and_recycles_buffers() {
        let dir = std::env::temp_dir().join(format!("soulwhistle-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rec.wav");
        let mut writer = std::io::BufWriter::new(File::create(&path).unwrap());
        writer.write_all(&[0u8; 44]).unwrap(); // header placeholder

        let (tx, rx) = std::sync::mpsc::sync_channel::<RecordingMsg>(4);
        let (pool_tx, pool_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(4);
        let handle = std::thread::spawn(move || run_recording_writer(writer, rx, pool_tx));

        // Two blocks of 100 stereo frames each (4 bytes per frame).
        tx.send(RecordingMsg::Data(vec![1u8; 400])).unwrap();
        tx.send(RecordingMsg::Data(vec![2u8; 400])).unwrap();
        tx.send(RecordingMsg::Stop).unwrap();
        handle.join().unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 44 + 800);
        let riff = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let data = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        assert_eq!(data, 800);
        assert_eq!(riff, 800 + 36);
        // Both buffers came back to the pool for reuse.
        assert_eq!(pool_rx.try_iter().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
