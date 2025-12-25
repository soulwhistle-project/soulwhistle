use rand::prelude::*;
use rand::rngs::SmallRng;

use serde::{Deserialize, Serialize};
use crate::coherence::CoherenceParams;
use crate::constants::*;
use crate::utils::generate_waveform;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum SignalType {
    // Carrier variants
    SchumannAM, // 100Hz AM by 7.83Hz
    SchumannFM, // 100Hz FM by 7.83Hz
    Schumann783AM, // 783Hz AM by 7.83Hz (Brycehelm variant)
    Sine100Hz,

    // Standard waves
    Sine,
    Triangle,
    Square,
    Saw,

    // Noise
    WhiteNoise,
    PinkNoise,
    LfoBreathing, // White noise + LFO

    // Chirps
    OrganicChirp,
    SyntheticChirp,

    // RF Modes
    WBFM, // +/- 75kHz
    NBFM, // +/- 12.5kHz
    AM,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioParams {
    // Preset Metadata (optional, not shown in UI mixer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,
    
    pub carrier_vol: f32, 
    pub carrier_type: SignalType,
    
    pub harmonic_vol: f32, 
    pub harmonic_type: SignalType,
    
    pub ping_vol: f32, 
    pub ping_type: SignalType,
    pub ping_freq_hz: f32,
    
    pub chirp_vol: f32, 
    pub chirp_type: SignalType,
    
    pub pad_vol: f32, 
    pub pad_type: SignalType,
    
    pub breath_vol: f32, 
    pub breath_type: SignalType,
    
    pub master_vol: f32,
    pub playing: bool,

    // HackRF Params
    pub rf_enabled: bool,
    pub rf_freq_hz: u64, // e.g. 100_000_000
    pub rf_gain: u32, // 0-47 typically
    pub rf_mode: SignalType, // WBFM, NBFM, AM
    pub rf_pulse_type: SignalType, // Sine, Square, Triangle, Saw (affects modulation waveform)
    pub rf_detected: bool, // Updated by RF worker
    
    // Signal Layer Lock
    pub lock_signal_layer: bool, // When true, signal layer controls are disabled
    
    // Network Streaming
    pub stream_enabled: bool,
    pub stream_port: u16,
    
    // Neural Coherence / Being Selection
    pub coherence: CoherenceParams,

    // Session tracking (updated by synthesizer, read-only for UI)
    #[serde(skip)]
    pub session_timer: f32,
    #[serde(skip)]
    pub session_phase: crate::coherence::SessionPhase,
}

#[derive(Default)]
struct SignalComponents {
    carrier: f32,
    harmonic: f32,
    ping: f32,
    chirp: f32,
    pad: f32,
    breath: f32,
}


impl Default for AudioParams {
    fn default() -> Self {
        Self {
            preset_title: None,
            preset_description: None,
            experimental: None,

            carrier_vol: 0.0,
            carrier_type: SignalType::SchumannAM,

            harmonic_vol: 0.0,
            harmonic_type: SignalType::Sine,

            ping_vol: 0.0,
            ping_type: SignalType::Sine,
            ping_freq_hz: DEFAULT_ULTRASONIC_PING_HZ,

            chirp_vol: 0.0,
            chirp_type: SignalType::OrganicChirp,

            pad_vol: 0.0,
            pad_type: SignalType::Sine,

            breath_vol: 0.0,
            breath_type: SignalType::LfoBreathing,

            master_vol: DEFAULT_MASTER_VOLUME,
            playing: true,

            rf_enabled: false,
            rf_freq_hz: HYDROGEN_LINE_HZ,
            rf_gain: DEFAULT_RF_GAIN_DB,
            rf_mode: SignalType::WBFM,
            rf_pulse_type: SignalType::Sine,
            rf_detected: false,

            lock_signal_layer: false,

            stream_enabled: false,
            stream_port: DEFAULT_STREAM_PORT,

            coherence: CoherenceParams::default(),

            session_timer: 0.0,
            session_phase: crate::coherence::SessionPhase::Startup,
        }
    }
}

/// Generates Neural Coherence binaural beats for left and right channels.
/// Binaural beats require HEADPHONES to be effective, as they rely on
/// projecting slightly different frequencies to each ear.
/// The core entrainment logic is based on the findings of Oster (1973),
/// which demonstrated the brain's frequency-following response.
/// See the main `CITATIONS.md` file for the full reference.
pub struct Synthesizer {
    sample_rate: f32,

    // Phases
    phase_100hz: f32,
    phase_783hz: f32,
    phase_7_83hz: f32,
    phase_528hz: f32,
    phase_17khz: f32,
    phase_432hz: f32,
    phase_2_5khz: f32,

    // Breath LFO
    breath_phase: f32,

    // Chirp logic
    chirp_timer: f32, // Seconds

    // Noise
    rng: SmallRng,

    // Neural Coherence binaural beat generator (public for session info access)
    pub coherence: crate::coherence::CoherenceSynth,
}

impl Synthesizer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase_100hz: 0.0,
            phase_783hz: 0.0,
            phase_7_83hz: 0.0,
            phase_528hz: 0.0,
            phase_17khz: 0.0,
            phase_432hz: 0.0,
            phase_2_5khz: 0.0,
            breath_phase: 0.0,
            chirp_timer: 0.0,
            rng: SmallRng::from_os_rng(),
            coherence: crate::coherence::CoherenceSynth::new(sample_rate),
        }
    }
    

    pub fn next_sample(&mut self, params: &AudioParams) -> (f32, f32) {
        if !params.playing {
            return (0.0, 0.0);
        }

        let dt = 1.0 / self.sample_rate;
        let components = self.generate_signal_components(params, dt);

        // Mix signal layer (mono)
        // Signal layer is muted when lock_signal_layer is true in a preset
        let signal_mixed = if params.lock_signal_layer {
            0.0
        } else {
            components.carrier * params.carrier_vol +
            components.harmonic * params.harmonic_vol +
            components.ping * params.ping_vol +
            components.chirp * params.chirp_vol +
            components.pad * params.pad_vol +
            components.breath * params.breath_vol
        };

        // Generate Neural Coherence binaural beat (stereo)
        let (coherence_left, coherence_right) = if !matches!(params.coherence.being_type, crate::coherence::BeingType::Unknown) {
            self.coherence.next_sample(&params.coherence)
        } else {
            (0.0, 0.0)
        };
        
        // Final stereo mix
        let left_out = (signal_mixed + coherence_left) * params.master_vol;
        let right_out = (signal_mixed + coherence_right) * params.master_vol;

        (left_out, right_out)
    }

    /// Centralized signal generation logic to be shared by audio and RF paths.
    fn generate_signal_components(&mut self, params: &AudioParams, dt: f32) -> SignalComponents {
        let pi2 = 2.0 * std::f32::consts::PI;
        let mut components = SignalComponents::default();

        // 1. Carrier
        self.phase_100hz = (self.phase_100hz + CARRIER_BASE_HZ * dt * pi2) % pi2;
        self.phase_783hz = (self.phase_783hz + CARRIER_783_HZ * dt * pi2) % pi2;
        self.phase_7_83hz = (self.phase_7_83hz + SCHUMANN_RESONANCE_HZ * dt * pi2) % pi2;

        components.carrier = match params.carrier_type {
            SignalType::SchumannAM => self.phase_100hz.sin() * (AM_MODULATION_MIN + AM_MODULATION_MIN * self.phase_7_83hz.sin()),
            SignalType::SchumannFM => {
                let _mod_freq = CARRIER_BASE_HZ + FM_MODULATION_RANGE_HZ * self.phase_7_83hz.sin();
                self.phase_100hz.sin()
            },
            SignalType::Schumann783AM => self.phase_783hz.sin() * (AM_MODULATION_MIN + AM_MODULATION_MIN * self.phase_7_83hz.sin()),
            SignalType::Sine100Hz => self.phase_100hz.sin(),
            SignalType::Square => if self.phase_100hz.sin() >= 0.0 { 1.0 } else { -1.0 },
            _ => self.phase_100hz.sin(),
        };

        // 2. Harmonic: 528 Hz (Solfeggio Mi)
        self.phase_528hz = (self.phase_528hz + SOLFEGGIO_MI_HZ * dt * pi2) % pi2;
        components.harmonic = generate_waveform(self.phase_528hz, params.harmonic_type);

        // 3. Ultrasonic Ping
        self.phase_17khz = (self.phase_17khz + params.ping_freq_hz * dt * pi2) % pi2;
        components.ping = generate_waveform(self.phase_17khz, params.ping_type);

        // 4. Chirps
        self.chirp_timer += dt;

        if self.chirp_timer > CHIRP_PERIOD_SECS {
            self.chirp_timer = 0.0;
        }

        if self.chirp_timer < CHIRP_DURATION_SECS {
            match params.chirp_type {
                SignalType::OrganicChirp => {
                    let fm_mod = (self.chirp_timer * CHIRP_FM_MOD_FACTOR).sin() * CHIRP_FM_MOD_RANGE_HZ;
                    let freq = CHIRP_BASE_HZ + fm_mod;
                    self.phase_2_5khz = (self.phase_2_5khz + freq * dt * pi2) % pi2;
                },
                SignalType::SyntheticChirp => {
                     let progress = self.chirp_timer / CHIRP_DURATION_SECS;
                     let freq = CHIRP_SWEEP_START_HZ + (CHIRP_SWEEP_END_HZ - CHIRP_SWEEP_START_HZ) * progress;
                     self.phase_2_5khz = (self.phase_2_5khz + freq * dt * pi2) % pi2;
                },
                _ => {
                    self.phase_2_5khz = (self.phase_2_5khz + CHIRP_BASE_HZ * dt * pi2) % pi2;
                }
            }


            let progress = self.chirp_timer / CHIRP_DURATION_SECS;
            let envelope = if progress < 0.5 { progress * 2.0 } else { 2.0 * (1.0 - progress) };
            
            let base = match params.chirp_type {
                 SignalType::Square => if self.phase_2_5khz.sin() >= 0.0 { 1.0 } else { -1.0 },
                 SignalType::Saw => {
                     let x = self.phase_2_5khz / pi2;
                     2.0 * (x - (x + 0.5).floor())
                 },
                 _ => self.phase_2_5khz.sin()
            };
            
            components.chirp = base * envelope;
        } else {
            self.phase_2_5khz = 0.0;
        }

        // 5. 432 Hz Ambient Pad (Verdi's A)
        self.phase_432hz = (self.phase_432hz + VERDI_A_HZ * dt * pi2) % pi2;
        components.pad = generate_waveform(self.phase_432hz, params.pad_type);

        // 6. Breath Layer: White noise shaped
        self.breath_phase = (self.breath_phase + BREATH_LFO_HZ * dt * pi2) % pi2;
        let noise: f32 = self.rng.random::<f32>() * 2.0 - 1.0;

        components.breath = match params.breath_type {
            SignalType::LfoBreathing => {
                 let breath_env = (AM_MODULATION_MIN + AM_MODULATION_MIN * self.breath_phase.sin()).powf(2.0);
                 noise * breath_env
            },
            SignalType::WhiteNoise => noise,
            SignalType::PinkNoise => noise * PINK_NOISE_FACTOR,
            SignalType::Sine => self.breath_phase.sin(), // LFO drone
            _ => noise
        };

        components
    }
    
    /// Generate RF-optimized signal with TRUE 7.83 Hz envelope (not AM trick)
    /// This is for electromagnetic transmission, not speaker playback
    pub fn next_rf_sample(&mut self, params: &AudioParams) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let components = self.generate_signal_components(params, dt);

        // For RF, we use a TRUE 7.83Hz envelope, not the AM trick for audio.
        let pi2 = 2.0 * std::f32::consts::PI;
        self.phase_7_83hz = (self.phase_7_83hz + SCHUMANN_RESONANCE_HZ * dt * pi2) % pi2;
        let schumann_envelope = AM_MODULATION_MIN + AM_MODULATION_MIN * self.phase_7_83hz.sin();
        
        let carrier_signal = schumann_envelope;

        // Mix all RF components
        let rf_signal = if params.lock_signal_layer {
            // In locked modes (Focus 10/12): Only transmit binaural beats
            let (hs_left, hs_right) = self.coherence.next_sample(&params.coherence);
            (hs_left + hs_right) * 0.5 // Mix to mono for RF
        } else {
            // In unlocked modes (UAP, Custom): Full signal layer
            carrier_signal * params.carrier_vol + 
            components.harmonic * params.harmonic_vol + 
            components.ping * params.ping_vol + 
            components.chirp * params.chirp_vol + 
            components.pad * params.pad_vol + 
            components.breath * params.breath_vol
        };
        
        rf_signal * params.master_vol
    }
}
