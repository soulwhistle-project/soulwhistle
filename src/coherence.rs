// Neural Coherence Binaural Beat Generator
// Based on decoded frequency maps (comparable to Gateway Experience)

use serde::{Deserialize, Serialize};
use crate::constants::*;

/// Session phase for progressive entrainment (Monroe-style multi-phase structure)
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum SessionPhase {
    Startup,        // Initial attention capture (0-2 min): ramp up volume
    Induction,      // Deep entrainment (2-15 min): full intensity
    Stabilization,  // Sustained state (15-25 min): stable with gamma bursts
    Return,         // Gradual return (25-30 min): ramp down to waking
}

/// Represents different consciousness states/beings
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum BeingType {
    Unknown,  // UAP frequencies (default Schumann/528Hz)
    HumanFocus10,  // "Mind Awake, Body Asleep" - 4.1 Hz theta
    HumanFocus12,  // "Expanded Awareness" - 1.5 Hz delta
    HumanFocus15,  // "No Time" - 4.8 Hz theta with 300 Hz carriers
    HumanFocus21,  // "Bridge to Other Realities" - 4.0 Hz theta/delta border
    HumanCustom,   // User-defined binaural beat
}

impl BeingType {
    /// Get the default preset filename for this BeingType
    pub fn default_preset_filename(&self) -> &'static str {
        match self {
            BeingType::Unknown => "DEFAULT_uap_frequencies.json",
            BeingType::HumanFocus10 => "DEFAULT_focus_10_mind_awake.json",
            BeingType::HumanFocus12 => "DEFAULT_focus_12_expanded.json",
            BeingType::HumanFocus15 => "DEFAULT_focus_15_no_time.json",
            BeingType::HumanFocus21 => "DEFAULT_focus_21_bridge.json",
            BeingType::HumanCustom => "DEFAULT_deep_focus_active.json",
        }
    }
}

/// Neural Coherence binaural beat parameters (based on decoded frequency maps)
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct CoherenceParams {
    pub enabled: bool,
    pub being_type: BeingType,

    // Carrier frequencies (Hz)
    pub left_carrier: f32,   // Left ear carrier
    pub right_carrier: f32,  // Right ear carrier

    // Derived: binaural_beat = |left - right|

    // Harmonics (optional - adds richness)
    pub harmonic_220hz: bool,  // Secondary harmonic cluster
    pub harmonic_495hz: bool,  // High harmonic (Focus 12 style)

    // Custom mode parameters
    pub custom_binaural_hz: f32,  // Target binaural beat (0.5-30 Hz)

    // Volume
    pub volume: f32,

    // Session phase timings (minutes) - configurable per preset
    pub startup_duration_min: f32,        // Duration of startup phase (default: 2 min)
    pub induction_duration_min: f32,      // Duration of induction phase (default: 13 min, ends at 15)
    pub stabilization_duration_min: f32,  // Duration of stabilization phase (default: 10 min, ends at 25)
    pub return_duration_min: f32,         // Duration of return phase (default: 5 min, ends at 30)
}

impl Default for CoherenceParams {
    fn default() -> Self {
        Self {
            enabled: false,
            being_type: BeingType::Unknown,
            left_carrier: OPTIMAL_CARRIER_HZ,
            right_carrier: OPTIMAL_CARRIER_HZ,
            harmonic_220hz: false,
            harmonic_495hz: false,
            custom_binaural_hz: DEFAULT_CUSTOM_BINAURAL_HZ,
            volume: DEFAULT_COHERENCE_VOLUME,
            // Default Monroe-style 30-minute session structure
            startup_duration_min: DEFAULT_STARTUP_DURATION_MIN,
            induction_duration_min: DEFAULT_INDUCTION_DURATION_MIN,
            stabilization_duration_min: DEFAULT_STABILIZATION_DURATION_MIN,
            return_duration_min: DEFAULT_RETURN_DURATION_MIN,
        }
    }
}

impl CoherenceParams {
    /// Set custom binaural beat frequency
    pub fn apply_custom_binaural(&mut self, target_hz: f32) {
        self.custom_binaural_hz = target_hz.clamp(BEAT_MIN_HZ, BEAT_MAX_HZ);

        // Use optimal carrier frequency, offset right channel
        self.left_carrier = OPTIMAL_CARRIER_HZ;
        self.right_carrier = OPTIMAL_CARRIER_HZ + self.custom_binaural_hz;

        // Enable moderate harmonics for custom mode
        self.harmonic_220hz = target_hz < THETA_MAX_HZ; // Only for theta/delta
        self.harmonic_495hz = target_hz < DELTA_MAX_HZ; // Only for deep delta
    }

    /// Get current binaural beat frequency
    pub fn binaural_beat_hz(&self) -> f32 {
        (self.left_carrier - self.right_carrier).abs()
    }

    /// Get brainwave state name
    pub fn brainwave_state(&self) -> &str {
        let hz = self.binaural_beat_hz();
        if hz < DELTA_MAX_HZ {
            "Delta (deep)"
        } else if hz < THETA_MAX_HZ {
            "Theta (meditation)"
        } else if hz < ALPHA_MAX_HZ {
            "Alpha (relaxed)"
        } else if hz < BETA_MAX_HZ {
            "Beta (alert)"
        } else {
            "Gamma (focus)"
        }
    }

    /// Get the end time for startup phase (in minutes)
    pub fn startup_end_min(&self) -> f32 {
        self.startup_duration_min
    }

    /// Get the end time for induction phase (in minutes)
    pub fn induction_end_min(&self) -> f32 {
        self.startup_duration_min + self.induction_duration_min
    }

    /// Get the end time for stabilization phase (in minutes)
    pub fn stabilization_end_min(&self) -> f32 {
        self.startup_duration_min + self.induction_duration_min + self.stabilization_duration_min
    }

    /// Get the total session duration (in minutes)
    pub fn total_session_min(&self) -> f32 {
        self.startup_duration_min + self.induction_duration_min +
        self.stabilization_duration_min + self.return_duration_min
    }
}

/// Neural Coherence binaural beat synthesizer
pub struct CoherenceSynth {
    sample_rate: f32,

    // Oscillator phases
    phase_left: f32,
    phase_right: f32,
    phase_harmonic_220: f32,
    phase_harmonic_495: f32,

    // Gamma burst interruption (Focus 10 technique)
    gamma_timer: f32,  // Tracks time since last gamma burst
    gamma_active: bool, // Whether we're currently in a gamma burst
    gamma_duration: f32, // How long the current gamma burst has been active

    // Multi-phase progression (Monroe-style session structure)
    // Use u64 sample counter to avoid f32 precision loss at high sample counts
    session_samples: u64,    // Total samples since session start
    current_phase: SessionPhase, // Current phase of the session
}

impl CoherenceSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase_left: 0.0,
            phase_right: 0.0,
            phase_harmonic_220: 0.0,
            phase_harmonic_495: 0.0,
            gamma_timer: 0.0,
            gamma_active: false,
            gamma_duration: 0.0,
            session_samples: 0,
            current_phase: SessionPhase::Startup,
        }
    }

    /// Get session timer in seconds (computed from sample count)
    fn session_timer_secs(&self) -> f32 {
        self.session_samples as f32 / self.sample_rate
    }
    
    /// Update session timer (called every sample regardless of being type)
    pub fn update_timer(&mut self, params: &CoherenceParams) {
        self.session_samples += 1;
        self.update_session_phase(params);
    }

    /// Generate next stereo sample pair
    pub fn next_sample(&mut self, params: &CoherenceParams) -> (f32, f32) {
        if !params.enabled {
            return (0.0, 0.0);
        }

        let dt = 1.0 / self.sample_rate;
        let pi2 = 2.0 * std::f32::consts::PI;
        let phase_volume = self.get_phase_volume_multiplier(params);

        // Gamma burst interruption for Focus 10 (Monroe technique)
        let (left_carrier, right_carrier) = if params.being_type == BeingType::HumanFocus10 {
            self.gamma_timer += dt;

            // Check if it's time for a gamma burst
            if self.gamma_timer >= GAMMA_BURST_INTERVAL_SECS && !self.gamma_active {
                self.gamma_active = true;
                self.gamma_duration = 0.0;
            }

            // If in gamma burst
            if self.gamma_active {
                self.gamma_duration += dt;

                // End gamma burst after duration
                if self.gamma_duration >= GAMMA_BURST_DURATION_SECS {
                    self.gamma_active = false;
                    self.gamma_timer = 0.0; // Reset for next cycle
                }

                // Generate gamma burst binaural beat
                (GAMMA_CARRIER_HZ, GAMMA_CARRIER_HZ + GAMMA_BEAT_HZ)
            } else {
                // Normal theta entrainment
                (params.left_carrier, params.right_carrier)
            }
        } else {
            // Other focus states: no gamma interruption
            (params.left_carrier, params.right_carrier)
        };

        // Update carrier oscillators
        self.phase_left = (self.phase_left + left_carrier * dt * pi2) % pi2;
        self.phase_right = (self.phase_right + right_carrier * dt * pi2) % pi2;
        
        // Generate carrier tones (pure sine waves for best binaural effect)
        let mut left = self.phase_left.sin();
        let mut right = self.phase_right.sin();
        
        // Add harmonics if enabled (like in real tapes)
        if params.harmonic_220hz {
            self.phase_harmonic_220 = (self.phase_harmonic_220 + HARMONIC_220_HZ * dt * pi2) % pi2;
            let harmonic = self.phase_harmonic_220.sin() * HARMONIC_220_RELATIVE_VOL;
            left += harmonic;
            right += harmonic;
        }

        if params.harmonic_495hz {
            self.phase_harmonic_495 = (self.phase_harmonic_495 + HARMONIC_495_HZ * dt * pi2) % pi2;
            let harmonic = self.phase_harmonic_495.sin() * HARMONIC_495_RELATIVE_VOL;
            left += harmonic;
            right += harmonic;
        }
        
        // Normalize and apply volume with phase-based ramping
        let volume = params.volume * phase_volume;
        left = left * volume;
        right = right * volume;

        (left, right)
    }

    /// Update session phase based on elapsed time (using configurable timings from params)
    fn update_session_phase(&mut self, params: &CoherenceParams) {
        let minutes = self.session_timer_secs() / 60.0;

        self.current_phase = if minutes < params.startup_end_min() {
            SessionPhase::Startup
        } else if minutes < params.induction_end_min() {
            SessionPhase::Induction
        } else if minutes < params.stabilization_end_min() {
            SessionPhase::Stabilization
        } else {
            SessionPhase::Return
        };
    }

    /// Calculate volume multiplier based on current phase (using configurable timings)
    /// Startup: 0.0 -> 1.0 (ramp up)
    /// Induction: 1.0 (full intensity)
    /// Stabilization: 1.0 (sustained)
    /// Return: 1.0 -> 0.3 (gentle ramp down)
    fn get_phase_volume_multiplier(&self, params: &CoherenceParams) -> f32 {
        let minutes = self.session_timer_secs() / 60.0;

        match self.current_phase {
            SessionPhase::Startup => {
                // Linear ramp from 0.0 to 1.0 over startup duration
                if params.startup_duration_min > 0.0 {
                    (minutes / params.startup_duration_min).min(1.0)
                } else {
                    1.0
                }
            },
            SessionPhase::Induction | SessionPhase::Stabilization => {
                // Full intensity
                1.0
            },
            SessionPhase::Return => {
                // Linear ramp from 1.0 to 0.3 over return duration
                let return_start = params.stabilization_end_min();
                if params.return_duration_min > 0.0 {
                    let progress = (minutes - return_start) / params.return_duration_min; // 0.0 to 1.0
                    1.0 - (progress * 0.7).min(0.7) // 1.0 -> 0.3
                } else {
                    0.3
                }
            },
        }
    }

    /// Reset all phases (useful when changing presets)
    pub fn reset(&mut self) {
        self.phase_left = 0.0;
        self.phase_right = 0.0;
        self.phase_harmonic_220 = 0.0;
        self.phase_harmonic_495 = 0.0;
        self.gamma_timer = 0.0;
        self.gamma_active = false;
        self.gamma_duration = 0.0;
        self.session_samples = 0;
        self.current_phase = SessionPhase::Startup;
    }

    /// Get current session information (for UI display)
    pub fn get_session_info(&self) -> (f32, SessionPhase) {
        (self.session_timer_secs(), self.current_phase)
    }
}
