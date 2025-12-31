use std::error::Error;
use std::fs::File;
use std::io::Write;
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
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod audio;
mod coherence;
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
    preset_desc_mode: PresetDescMode,

    // Mapping from UI index to original channel index (for collapse toggle)
    visible_channel_indices: Vec<usize>,

    // RF safety
    rf_disclaimer_shown: bool,
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
    // RF Controls
    RfEnable,
    RfFreq,
    RfGain,
    RfMode,
    RfPulseType,
    // Network Streaming
    StreamEnable,
    StreamPort,
    // UI Spacer
    Spacer,
}

impl App {
    fn new(params: Arc<Mutex<AudioParams>>, stream_client_count: Arc<Mutex<usize>>) -> Self {
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
            preset_desc_mode: PresetDescMode::Hidden,  // Start hidden to save space
            visible_channel_indices: Vec::new(),
            rf_disclaimer_shown: false,
            channels: vec![
                ChannelInfo { name: "Master Volume".to_string(), id: ChannelId::Master },
                ChannelInfo { name: "".to_string(), id: ChannelId::Spacer }, // Spacer
                ChannelInfo { name: "PRESET:".to_string(), id: ChannelId::BeingType },
                ChannelInfo { name: "PRESET_DESC".to_string(), id: ChannelId::PresetDescription },
                ChannelInfo { name: "".to_string(), id: ChannelId::Spacer }, // Spacer
                ChannelInfo { name: "BINAURAL BEATS".to_string(), id: ChannelId::CoherenceVol },
                ChannelInfo { name: "  Volume".to_string(), id: ChannelId::CoherenceVol },
                ChannelInfo { name: "  Beat Adjust".to_string(), id: ChannelId::BinauralAdjust },
                ChannelInfo { name: "  Session Progress".to_string(), id: ChannelId::SessionTimer },
                ChannelInfo { name: "".to_string(), id: ChannelId::Spacer }, // Spacer
                ChannelInfo { name: "SIGNAL LAYER".to_string(), id: ChannelId::Carrier },
                ChannelInfo { name: "  Carrier (7.83Hz)".to_string(), id: ChannelId::Carrier },
                ChannelInfo { name: "  Harmonic (528Hz)".to_string(), id: ChannelId::Harmonic },
                ChannelInfo { name: "  Ultrasonic Ping".to_string(), id: ChannelId::Ping },
                ChannelInfo { name: "  Ping Frequency".to_string(), id: ChannelId::PingFreq },
                ChannelInfo { name: "  Organic Chirps".to_string(), id: ChannelId::Chirp },
                ChannelInfo { name: "  Ambient Pad (432Hz)".to_string(), id: ChannelId::Pad },
                ChannelInfo { name: "  Breath Layer".to_string(), id: ChannelId::Breath },
                ChannelInfo { name: "".to_string(), id: ChannelId::Spacer }, // Spacer
                ChannelInfo { name: "HACKRF TRANSMIT".to_string(), id: ChannelId::RfEnable },
                ChannelInfo { name: "  RF Enable".to_string(), id: ChannelId::RfEnable },
                ChannelInfo { name: "  RF Frequency".to_string(), id: ChannelId::RfFreq },
                ChannelInfo { name: "  RF Gain (VGA)".to_string(), id: ChannelId::RfGain },
                ChannelInfo { name: "  RF Modulation Mode".to_string(), id: ChannelId::RfMode },
                ChannelInfo { name: "  RF Pulse Waveform".to_string(), id: ChannelId::RfPulseType },
                ChannelInfo { name: "".to_string(), id: ChannelId::Spacer }, // Spacer
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
                // Navigate through visible items only
                if self.visible_channel_indices.is_empty() { return; }
                let current_visual_idx = self.state.selected().unwrap_or(0);
                let next_visual_idx = cycle_index(current_visual_idx, self.visible_channel_indices.len(), 1);
                self.state.select(Some(next_visual_idx));
            },
            AppMode::PresetSelect => {
                if self.preset_list.is_empty() { return; }
                let i = cycle_index(self.preset_state.selected().unwrap_or(0), self.preset_list.len(), 1);
                self.preset_state.select(Some(i));
            }
        }
    }

    fn previous(&mut self) {
        match self.mode {
            AppMode::Mixer => {
                // Navigate through visible items only
                if self.visible_channel_indices.is_empty() { return; }
                let current_visual_idx = self.state.selected().unwrap_or(0);
                let prev_visual_idx = cycle_index(current_visual_idx, self.visible_channel_indices.len(), -1);
                self.state.select(Some(prev_visual_idx));
            },
            AppMode::PresetSelect => {
                if self.preset_list.is_empty() { return; }
                let i = cycle_index(self.preset_state.selected().unwrap_or(0), self.preset_list.len(), -1);
                self.preset_state.select(Some(i));
            }
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

                            loaded.stream_enabled = stream_enabled;
                            loaded.stream_port = stream_port;

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

                ChannelId::Spacer => {
                    // Spacer lines do nothing when arrows are pressed
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

                        loaded.stream_enabled = stream_enabled;
                        loaded.stream_port = stream_port;

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

    // Copy embedded presets to user directory if they don't exist
    for preset in embedded_presets::EMBEDDED_PRESETS {
        let preset_path = presets_dir.join(preset.filename);

        // Only copy if the file doesn't already exist (user may have modified it)
        if !preset_path.exists() {
            std::fs::write(&preset_path, preset.content)?;
        }
    }

    Ok(())
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
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate() as f32;

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
                    stream_client_count_for_server.clone()
                );
                server.run(); // Blocking call - only returns if server fails or stream disabled
            }
            
            // If not enabled or server stopped, wait and check again
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    // Run audio in a separate thread (handled by cpal stream)

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), audio_params, stream_buffer_for_audio, sample_rate),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), audio_params, stream_buffer_for_audio, sample_rate),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), audio_params, stream_buffer_for_audio, sample_rate),
        _ => panic!("Unsupported sample format"),
    }?;

    stream.play()?;

    // 2. TUI Setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(params, stream_client_count_for_app);
    app.current_preset = loaded_preset_name;
    app.refresh_presets();
    
    let res = run_app(&mut terminal, app, error_rx);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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
    sample_rate: f32,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::Sample + cpal::FromSample<f32> + cpal::SizedSample,
{
    let mut synth = Synthesizer::new(sample_rate);
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // We lock once per buffer to get latest params. 
            // For smoother updates we could copy the params, but locking per sample is too slow.
            // Actually, we should probably lock just to copy the struct and then run the loop.
            let p = { params.lock().clone() }; // Copy AudioParams (it needs to be Copy/Clone or manual copy)
            
            for frame in data.chunks_mut(channels) {
                let (left_sample_f32, right_sample_f32) = synth.next_sample(&p);

                // Push to streaming buffer if enabled
                if p.stream_enabled {
                    stream_buffer.push_samples(left_sample_f32, right_sample_f32);
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

            // Update session info in params (once per buffer for efficiency)
            let (session_timer, session_phase) = synth.coherence.get_session_info();
            let mut params_write = params.lock();
            params_write.session_timer = session_timer;
            params_write.session_phase = session_phase;
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App, error_rx: std::sync::mpsc::Receiver<String>) -> std::io::Result<()> {
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
                                KeyCode::Char('l') => app.enter_preset_mode(),
                                KeyCode::Char('o') => app.cycle_modulation(),
                                KeyCode::Char('m') => app.toggle_mute(),
                                KeyCode::Char(' ') => app.toggle_playback(),
                                KeyCode::Char('x') => app.toggle_collapse(),
                                KeyCode::Down | KeyCode::Char('j') => app.next(),
                                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                                KeyCode::Left | KeyCode::Char('h') => app.adjust_volume(-0.01),
                                KeyCode::Right => app.adjust_volume(0.01),
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
    
    let line1 = format!(
        "{} {} | Master: {:.0}% | {} {} | {}",
        playback_icon,
        if params.playing { "Playing" } else { "Paused" },
        params.master_vol * 100.0,
        being_icon,
        being_short,
        rf_status
    );
    
    // Line 2: Keybindings | Status message
    let mut line2 = "[m]ute [o]scillator [Space]pause [q]uit".to_string();

    if let Some((text, time)) = &app.status_msg {
        // Show warnings (⚠️) for longer
        let timeout = if text.starts_with("⚠️") { STATUS_WARNING_TIMEOUT_SECS } else { STATUS_TIMEOUT_SECS };
        if time.elapsed() < std::time::Duration::from_secs(timeout) {
            line2 = format!("[m]ute [o]scillator [Space]pause [q]uit | STATUS: {}", text);
        }
    }
    
    drop(params); // Release lock
    
    let status_text = format!("{}\n{}", line1, line2);
    let instructions = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(instructions, chunks[1]);
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
    // Get current params to display
    let params = app.params.lock(); // This lock is quick, just for reading

    // Add header items
    let mut all_items = vec![
        ListItem::new(Line::from(" ███████╗ ██████╗ ██╗   ██╗██╗     ██╗    ██╗██╗  ██╗██╗███████╗████████╗██╗     ███████╗")),
        ListItem::new(Line::from(" ██╔════╝██╔═══██╗██║   ██║██║     ██║    ██║██║  ██║██║██╔════╝╚══██╔══╝██║     ██╔════╝")),
        ListItem::new(Line::from(" ███████╗██║   ██║██║   ██║██║     ██║ █╗ ██║███████║██║███████╗   ██║   ██║     █████╗  ")),
        ListItem::new(Line::from(" ╚════██║██║   ██║██║   ██║██║     ██║███╗██║██╔══██║██║╚════██║   ██║   ██║     ██╔══╝  ")),
        ListItem::new(Line::from(" ███████║╚██████╔╝╚██████╔╝███████╗╚███╔███╔╝██║  ██║██║███████║   ██║   ███████╗███████╗")),
        ListItem::new(Line::from(" ╚══════╝ ╚═════╝  ╚═════╝ ╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝╚══════╝")),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from("         Multi-Dimensional Communication System • Signal/Consciousness Explorer")),
        ListItem::new(Line::from("         ─────────────────────────────────────────────────────────────────────────────")),
    ];

    // Build visible channel indices mapping and items list
    let items_with_indices: Vec<(usize, ListItem)> = app
        .channels
        .iter()
        .enumerate()
        .filter_map(|(idx, chan)| {
            // Empty lines are spacers - always show them
            if chan.name.is_empty() {
                return Some((idx, ListItem::new(Line::from(""))));
            }

            // Skip collapsed section children (indented items after section headers)
            // Look backwards to find the nearest section header
            if chan.name.starts_with("  ") {
                // This is a child item - find its parent section
                for i in (0..idx).rev() {
                    let parent = &app.channels[i];
                    if !parent.name.starts_with("  ") && !parent.name.is_empty() {
                        // Found the parent section header
                        if parent.name == "SIGNAL LAYER" && app.signal_layer_collapsed {
                            return None;
                        }
                        if parent.name == "HACKRF TRANSMIT" && app.hackrf_collapsed {
                            return None;
                        }
                        if parent.name.starts_with("NETWORK STREAMING") && app.streaming_collapsed {
                            return None;
                        }
                        break;
                    }
                }
            }

            // Section headers - special handling for PRESET: and BINAURAL BEATS
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
                let experimental_tag = if params.experimental == Some(true) {
                    "[EXPERIMENTAL] "
                } else {
                    ""
                };
                let indicator = match app.preset_desc_mode {
                    PresetDescMode::Hidden => "[+]",
                    PresetDescMode::Brief => "[-]",
                    PresetDescMode::Full => "[=]",
                };
                return Some((idx, ListItem::new(Line::from(format!("PRESET: {}{} {}                    [s]ave [l]oad  [x] info", experimental_tag, preset_name, indicator)))));
            }

            if chan.name == "PRESET_DESC" {
                // Show description based on mode
                match app.preset_desc_mode {
                    PresetDescMode::Hidden => return None, // Don't show anything
                    PresetDescMode::Brief => {
                        // Show first 3 lines with "..."
                        if let Some(ref description) = params.preset_description {
                            let mut lines = wrap_text(description, DESC_MAX_WIDTH, Some(DESC_BRIEF_LINES), "  ");

                            if description.len() > DESC_BRIEF_CHAR_THRESHOLD {
                                lines.push("  ... (press [x] on PRESET line for full text)".to_string());
                            }

                            return Some((idx, ListItem::new(lines.join("\n"))));
                        } else {
                            return None;
                        }
                    },
                    PresetDescMode::Full => {
                        // Show full description
                        if let Some(ref description) = params.preset_description {
                            let lines = wrap_text(description, DESC_MAX_WIDTH, None, "  ");
                            return Some((idx, ListItem::new(lines.join("\n"))));
                        } else {
                            return None;
                        }
                    }
                }
            }

            if chan.name == "BINAURAL BEATS" {
                let headphones_note = if !matches!(params.coherence.being_type, crate::coherence::BeingType::Unknown) {
                    "     [🎧 HEADPHONES REQUIRED]"
                } else {
                    ""
                };
                return Some((idx, ListItem::new(Line::from(format!("BINAURAL BEATS{}", headphones_note)))));
            }

            // Other section headers with collapse indicators
            if chan.name == "SIGNAL LAYER" {
                let indicator = if app.signal_layer_collapsed { "[+]" } else { "[-]" };
                let item_count = 7; // carrier, harmonic, ping, ping_freq, chirp, pad, breath
                let status = if app.signal_layer_collapsed {
                    format!("{} items hidden", item_count)
                } else {
                    "expanded".to_string()
                };
                return Some((idx, ListItem::new(Line::from(format!("{} {} ({})            Press [x] to toggle", chan.name, indicator, status)))));
            }
            if chan.name == "HACKRF TRANSMIT" {
                let indicator = if app.hackrf_collapsed { "[+]" } else { "[-]" };
                let item_count = 5; // enable, freq, gain, mode, pulse_type
                let status = if app.hackrf_collapsed {
                    format!("{} items hidden", item_count)
                } else {
                    "expanded".to_string()
                };
                return Some((idx, ListItem::new(Line::from(format!("{} {} ({})            Press [x] to toggle", chan.name, indicator, status)))));
            }
            if chan.name == "NETWORK STREAMING [EXPERIMENTAL]" || chan.name == "NETWORK STREAMING" {
                let indicator = if app.streaming_collapsed { "[+]" } else { "[-]" };
                let item_count = 2; // enable, port
                let status = if app.streaming_collapsed {
                    format!("{} items hidden", item_count)
                } else {
                    "expanded".to_string()
                };
                return Some((idx, ListItem::new(Line::from(format!("{} {} ({})            Press [x] to toggle", chan.name, indicator, status)))));
            }

            let content = match chan.id {
                ChannelId::BeingType => {
                    // This is now handled in the header section above
                    String::new()
                },
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
                    format!("{:<40} {:.2}kHz (arrows to adjust)", 
                        chan.name, params.ping_freq_hz / 1000.0)
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
                    let warning = if params.rf_enabled && !params.rf_detected {
                        " NO DEVICE!"
                    } else {
                        ""
                    };
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
                        _ => (0.0, None),
                    };
                    
                    let is_signal_channel = matches!(chan.id, 
                        ChannelId::Carrier | ChannelId::Harmonic | ChannelId::Ping | 
                        ChannelId::Chirp | ChannelId::Pad | ChannelId::Breath);
                    
                    let filled = (vol * 20.0) as usize;
                    let bar: String = std::iter::repeat("█").take(filled).collect();
                    let empty: String = std::iter::repeat("░").take(20 - filled).collect();
                    
                    let suffix = if is_signal_channel && params.lock_signal_layer {
                        " [LOCKED]"
                    } else {
                        ""
                    };
                    
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

    // Store the mapping in app for toggle_collapse to use
    drop(params); // Release lock before modifying app
    app.visible_channel_indices = visible_indices;

    // Combine header and channel items
    all_items.extend(items);

    let list = List::new(all_items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        .highlight_symbol(">> ");

    // Offset the state to account for header items
    let mut adjusted_state = app.state.clone();
    if let Some(selected) = app.state.selected() {
        adjusted_state.select(Some(selected + MIXER_HEADER_OFFSET));
    }

    f.render_stateful_widget(list, area, &mut adjusted_state);
}
