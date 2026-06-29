use rand::prelude::*;
use rand::rngs::SmallRng;

use serde::{Deserialize, Serialize};
use crate::coherence::CoherenceParams;
use crate::constants::*;
use crate::utils::{generate_waveform, fast_sin, soft_clip};

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

    // Preset change tracking (for detecting when to reset synthesizer state)
    #[serde(skip)]
    pub preset_version: u32,
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
            preset_version: 0,
        }
    }
}

impl AudioParams {
    /// Clone only the numeric fields needed for audio synthesis.
    /// Avoids heap-allocating String fields (preset_title, preset_description)
    /// which are only needed for UI display.
    pub fn clone_for_audio(&self) -> Self {
        Self {
            preset_title: None,
            preset_description: None,
            experimental: None,
            carrier_vol: self.carrier_vol,
            carrier_type: self.carrier_type,
            harmonic_vol: self.harmonic_vol,
            harmonic_type: self.harmonic_type,
            ping_vol: self.ping_vol,
            ping_type: self.ping_type,
            ping_freq_hz: self.ping_freq_hz,
            chirp_vol: self.chirp_vol,
            chirp_type: self.chirp_type,
            pad_vol: self.pad_vol,
            pad_type: self.pad_type,
            breath_vol: self.breath_vol,
            breath_type: self.breath_type,
            master_vol: self.master_vol,
            playing: self.playing,
            rf_enabled: self.rf_enabled,
            rf_freq_hz: self.rf_freq_hz,
            rf_gain: self.rf_gain,
            rf_mode: self.rf_mode,
            rf_pulse_type: self.rf_pulse_type,
            rf_detected: self.rf_detected,
            lock_signal_layer: self.lock_signal_layer,
            stream_enabled: self.stream_enabled,
            stream_port: self.stream_port,
            coherence: self.coherence,
            session_timer: self.session_timer,
            session_phase: self.session_phase,
            preset_version: self.preset_version,
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

    // DC offset removal high-pass filter state (per channel)
    dc_prev_in_l: f32,
    dc_prev_out_l: f32,
    dc_prev_in_r: f32,
    dc_prev_out_r: f32,
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
            dc_prev_in_l: 0.0,
            dc_prev_out_l: 0.0,
            dc_prev_in_r: 0.0,
            dc_prev_out_r: 0.0,
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

        // Always update session timer (regardless of being type)
        self.coherence.update_timer(&params.coherence);

        // Generate Neural Coherence binaural beat (stereo)
        let (coherence_left, coherence_right) = if !matches!(params.coherence.being_type, crate::coherence::BeingType::Unknown) {
            self.coherence.next_sample(&params.coherence)
        } else {
            (0.0, 0.0)
        };
        
        // Final stereo mix with soft clipping and DC offset removal
        let left_raw = (signal_mixed + coherence_left) * params.master_vol;
        let right_raw = (signal_mixed + coherence_right) * params.master_vol;

        // Soft clip to prevent harsh digital distortion
        let left_clipped = soft_clip(left_raw);
        let right_clipped = soft_clip(right_raw);

        // DC offset removal: y[n] = α * (y[n-1] + x[n] - x[n-1])
        let left_out = DC_FILTER_ALPHA
            * (self.dc_prev_out_l + left_clipped - self.dc_prev_in_l);
        self.dc_prev_in_l = left_clipped;
        self.dc_prev_out_l = left_out;

        let right_out = DC_FILTER_ALPHA
            * (self.dc_prev_out_r + right_clipped - self.dc_prev_in_r);
        self.dc_prev_in_r = right_clipped;
        self.dc_prev_out_r = right_out;

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
            SignalType::SchumannAM => fast_sin(self.phase_100hz) * (AM_MODULATION_MIN + AM_MODULATION_MIN * fast_sin(self.phase_7_83hz)),
            SignalType::SchumannFM => {
                // True frequency modulation via phase modulation (spectrally equivalent):
                // sin(carrier_phase + beta * sin(mod_phase)), beta = deviation / mod_freq.
                let beta = FM_MODULATION_RANGE_HZ / SCHUMANN_RESONANCE_HZ;
                fast_sin(self.phase_100hz + beta * fast_sin(self.phase_7_83hz))
            },
            SignalType::Schumann783AM => fast_sin(self.phase_783hz) * (AM_MODULATION_MIN + AM_MODULATION_MIN * fast_sin(self.phase_7_83hz)),
            SignalType::Sine100Hz => fast_sin(self.phase_100hz),
            SignalType::Square => if fast_sin(self.phase_100hz) >= 0.0 { 1.0 } else { -1.0 },
            _ => fast_sin(self.phase_100hz),
        };

        // 2. Harmonic: 528 Hz (Solfeggio Mi)
        self.phase_528hz = (self.phase_528hz + SOLFEGGIO_MI_HZ * dt * pi2) % pi2;
        components.harmonic = generate_waveform(self.phase_528hz, params.harmonic_type);

        // 3. Ultrasonic Ping
        // Nyquist guard: a ping above the Nyquist limit can't be reproduced and would
        // alias down to an audible tone (e.g. 40 kHz @ 48 kHz -> 8 kHz). Mute it instead
        // of emitting a false frequency. Margin keeps it clear of the fold point.
        self.phase_17khz = (self.phase_17khz + params.ping_freq_hz * dt * pi2) % pi2;
        let nyquist_limit = self.sample_rate * NYQUIST_SAFE_FRACTION;
        components.ping = if params.ping_freq_hz > nyquist_limit {
            0.0
        } else {
            generate_waveform(self.phase_17khz, params.ping_type)
        };

        // 4. Chirps
        self.chirp_timer += dt;

        if self.chirp_timer > CHIRP_PERIOD_SECS {
            self.chirp_timer = 0.0;
        }

        if self.chirp_timer < CHIRP_DURATION_SECS {
            match params.chirp_type {
                SignalType::OrganicChirp => {
                    let fm_mod = fast_sin(self.chirp_timer * CHIRP_FM_MOD_FACTOR) * CHIRP_FM_MOD_RANGE_HZ;
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
                 SignalType::Square => if fast_sin(self.phase_2_5khz) >= 0.0 { 1.0 } else { -1.0 },
                 SignalType::Saw => {
                     let x = self.phase_2_5khz / pi2;
                     2.0 * (x - (x + 0.5).floor())
                 },
                 _ => fast_sin(self.phase_2_5khz)
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
                 let breath_env = AM_MODULATION_MIN + AM_MODULATION_MIN * fast_sin(self.breath_phase);
                 noise * breath_env * breath_env
            },
            SignalType::WhiteNoise => noise,
            SignalType::PinkNoise => noise * PINK_NOISE_FACTOR,
            SignalType::Sine => fast_sin(self.breath_phase), // LFO drone
            _ => noise
        };

        components
    }
    
    /// Generate RF-optimized signal with TRUE 7.83 Hz envelope (not AM trick)
    /// This is for electromagnetic transmission, not speaker playback
    pub fn next_rf_sample(&mut self, params: &AudioParams) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let components = self.generate_signal_components(params, dt);

        // Advance the session timer every sample so phase-based volume ramping and
        // gamma bursts progress on the RF path too (audio path does this in next_sample).
        self.coherence.update_timer(&params.coherence);

        // For RF, we use a TRUE 7.83Hz envelope, not the AM trick for audio.
        // phase_7_83hz was already advanced inside generate_signal_components — reuse it
        // (advancing again here would double the envelope frequency to 15.66 Hz).
        let schumann_envelope = AM_MODULATION_MIN + AM_MODULATION_MIN * fast_sin(self.phase_7_83hz);

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

    /// Reset all synthesizer state (called when preset changes to avoid glitches)
    pub fn reset(&mut self) {
        // Reset all phase accumulators
        self.phase_100hz = 0.0;
        self.phase_783hz = 0.0;
        self.phase_7_83hz = 0.0;
        self.phase_528hz = 0.0;
        self.phase_17khz = 0.0;
        self.phase_432hz = 0.0;
        self.phase_2_5khz = 0.0;
        self.breath_phase = 0.0;
        self.chirp_timer = 0.0;

        // Reset DC filter state
        self.dc_prev_in_l = 0.0;
        self.dc_prev_out_l = 0.0;
        self.dc_prev_in_r = 0.0;
        self.dc_prev_out_r = 0.0;

        // Reset coherence synthesizer (session timer, gamma bursts, etc.)
        self.coherence.reset();
    }
}
