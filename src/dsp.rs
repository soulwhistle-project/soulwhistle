//! Minimal DSP building blocks for tailor-made notched sound therapy.
//!
//! Notched sound therapy (Okamoto et al. 2010, PNAS; Pantev/Stein follow-ups)
//! removes a band of energy centered on the listener's tinnitus pitch. Neurons at
//! the notch edges fire normally and laterally inhibit the over-active
//! tinnitus-frequency neurons, reducing perceived loudness over weeks of daily use.

use crate::constants::NYQUIST_SAFE_FRACTION;

/// Transposed-direct-form-II biquad filter.
#[derive(Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Identity (pass-through) biquad.
    pub fn passthrough() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }

    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    /// RBJ cookbook band-reject (notch). `bw_oct` is the -3 dB bandwidth in octaves.
    /// Gain is theoretically -∞ at `fc`, rising back to unity outside the band.
    pub fn set_notch(&mut self, fs: f32, fc: f32, bw_oct: f32) {
        let w0 = 2.0 * std::f32::consts::PI * fc / fs;
        let (sin, cos) = w0.sin_cos();
        // BW-in-octaves form of alpha (RBJ cookbook).
        let ln2_2 = std::f32::consts::LN_2 / 2.0;
        let alpha = sin * (ln2_2 * bw_oct * w0 / sin).sinh();
        let a0 = 1.0 + alpha;
        self.b0 = 1.0 / a0;
        self.b1 = (-2.0 * cos) / a0;
        self.b2 = 1.0 / a0;
        self.a1 = (-2.0 * cos) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Number of cascaded notch biquads — deepens the trough and steepens the skirts.
const NOTCH_STAGES: usize = 2;

/// Band-reject (notch) filter: a cascade of RBJ notch biquads centered on the
/// tinnitus pitch. Removes energy at the pitch while passing the notch edges,
/// which is what drives lateral inhibition in notched sound therapy.
pub struct BandReject {
    stages: [Biquad; NOTCH_STAGES],
}

impl BandReject {
    pub fn new() -> Self {
        Self { stages: [Biquad::passthrough(); NOTCH_STAGES] }
    }

    /// Configure the notch: `center_hz` is the tinnitus pitch, `width_oct` the notch
    /// width in octaves. Center is clamped to a safe range below Nyquist.
    pub fn set_notch(&mut self, fs: f32, center_hz: f32, width_oct: f32) {
        let nyquist_limit = fs * NYQUIST_SAFE_FRACTION;
        let fc = center_hz.clamp(20.0, nyquist_limit);
        for b in &mut self.stages {
            b.set_notch(fs, fc, width_oct);
        }
    }

    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let mut y = x;
        for b in &mut self.stages {
            y = b.process(y);
        }
        y
    }

    pub fn reset(&mut self) {
        for b in &mut self.stages {
            b.reset();
        }
    }
}

/// Pink (1/f) noise generator — Paul Kellet's refined economical filter applied to
/// a white-noise input. Output is roughly unity-RMS, in [-1, 1] for white in [-1, 1].
pub struct PinkNoise {
    b: [f32; 7],
}

impl PinkNoise {
    pub fn new() -> Self {
        Self { b: [0.0; 7] }
    }

    #[inline(always)]
    pub fn process(&mut self, white: f32) -> f32 {
        let b = &mut self.b;
        b[0] = 0.99886 * b[0] + white * 0.0555179;
        b[1] = 0.99332 * b[1] + white * 0.0750759;
        b[2] = 0.96900 * b[2] + white * 0.1538520;
        b[3] = 0.86650 * b[3] + white * 0.3104856;
        b[4] = 0.55000 * b[4] + white * 0.5329522;
        b[5] = -0.7616 * b[5] - white * 0.0168980;
        let pink = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + white * 0.5362;
        b[6] = white * 0.115926;
        // Scale to keep amplitude in a comparable range to the white input.
        pink * 0.11
    }

    pub fn reset(&mut self) {
        self.b = [0.0; 7];
    }
}
