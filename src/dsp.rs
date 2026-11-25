//! Minimal DSP building blocks for tailor-made notched sound therapy.
//!
//! Notched sound therapy (Okamoto et al. 2010, PNAS; Pantev/Stein follow-ups)
//! removes a band of energy centered on the listener's tinnitus pitch. The proposed
//! mechanism is lateral inhibition of the over-active tinnitus-frequency neurons by
//! neurons at the notch edges; clinical evidence is mixed (CITATIONS.md, 35 to 39).

use crate::constants::NYQUIST_SAFE_FRACTION;
use num_complex::Complex64;

/// Order of the analog low-pass prototype; the band-stop has this many biquads.
/// Order 8 brings the response back within 3 dB about 0.08 octave outside each
/// band edge, so the removed band really is the width the user set.
const NOTCH_ORDER: usize = 8;

/// Minimum attenuation across the whole notch band (dB).
pub const NOTCH_STOPBAND_DB: f64 = 30.0;

/// One second-order section in f64. The notch sections are high-Q at narrow
/// widths, where f32 coefficients and state would add audible noise.
#[derive(Clone, Copy)]
struct Section {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Section {
    const PASS: Section = Section { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 };

    #[inline(always)]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Band-reject filter for notched sound therapy: an order-8 inverse Chebyshev
/// (Chebyshev type II) band-stop.
///
/// The band `[center / 2^(w/2), center * 2^(w/2)]` (`w` = width in octaves, so the
/// notch is centred on the tinnitus pitch on a log scale) is attenuated by at least
/// `NOTCH_STOPBAND_DB` everywhere inside it, and the response is maximally flat
/// outside it. The previous two-stage RBJ notch only reached 20 dB over about a
/// third of the labelled width.
pub struct BandReject {
    stages: [Section; NOTCH_ORDER],
}

impl BandReject {
    pub fn new() -> Self {
        Self { stages: [Section::PASS; NOTCH_ORDER] }
    }

    /// Configure the notch: `center_hz` is the tinnitus pitch, `width_oct` the notch
    /// width in octaves. The band is kept between 20 Hz and a safe limit below Nyquist.
    /// Allocation-free; safe to call from the audio callback when parameters change.
    pub fn set_notch(&mut self, fs: f32, center_hz: f32, width_oct: f32) {
        let fs = fs as f64;
        let limit = fs * NYQUIST_SAFE_FRACTION as f64;
        let fc = (center_hz as f64).clamp(20.0, limit);
        let half = 2f64.powf(width_oct.max(0.01) as f64 / 2.0);
        let f_lo = (fc / half).max(10.0);
        let f_hi = (fc * half).min(limit);
        self.stages = design_cheby2_bandstop(fs, f_lo, f_hi, NOTCH_STOPBAND_DB);
    }

    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let mut y = x as f64;
        for s in &mut self.stages {
            y = s.process(y);
        }
        y as f32
    }

    pub fn reset(&mut self) {
        for s in &mut self.stages {
            s.z1 = 0.0;
            s.z2 = 0.0;
        }
    }

    /// Magnitude response in dB at `f` Hz (for tests and diagnostics).
    #[cfg(test)]
    pub fn response_db(&self, fs: f32, f: f32) -> f64 {
        let w = 2.0 * std::f64::consts::PI * f as f64 / fs as f64;
        let z1 = Complex64::from_polar(1.0, -w);
        let z2 = z1 * z1;
        let mut h = Complex64::new(1.0, 0.0);
        for s in &self.stages {
            h *= (z2 * s.b2 + z1 * s.b1 + s.b0) / (z2 * s.a2 + z1 * s.a1 + 1.0);
        }
        20.0 * h.norm().log10()
    }
}

/// Digital Chebyshev type II band-stop, stopband `[f_lo, f_hi]` Hz, attenuation at
/// least `atten_db` inside it, unity gain at DC. Standard design: analog low-pass
/// prototype poles and zeros, low-pass to band-stop transform, bilinear transform
/// with pre-warped band edges. Fixed-size arrays only (no allocation).
fn design_cheby2_bandstop(fs: f64, f_lo: f64, f_hi: f64, atten_db: f64) -> [Section; NOTCH_ORDER] {
    use std::f64::consts::PI;
    const N: usize = NOTCH_ORDER;
    const HALF: usize = NOTCH_ORDER / 2; // N is even: N/2 conjugate pole pairs

    // Pre-warp the band edges for the bilinear transform.
    let k = 2.0 * fs;
    let w1 = k * (PI * f_lo / fs).tan();
    let w2 = k * (PI * f_hi / fs).tan();
    let w0sq = w1 * w2;
    let bw = w2 - w1;

    // Chebyshev II prototype, stopband edge at 1 rad/s.
    let eps = 1.0 / (10f64.powf(atten_db / 10.0) - 1.0).sqrt();
    let mu = (1.0 / eps).asinh() / N as f64;

    // Band-stop poles and zeros in the upper half plane (each LP pole of the
    // upper half maps to two band-stop roots; their conjugates come from the
    // conjugate LP pole, so the upper-half roots fully describe the filter).
    let mut poles = [Complex64::new(0.0, 0.0); N];
    let mut zeros = [Complex64::new(0.0, 0.0); N];
    for i in 0..HALF {
        let theta = PI * (2 * i + 1) as f64 / (2 * N) as f64;
        // Chebyshev I pole, then invert for Chebyshev II.
        let p1 = Complex64::new(-mu.sinh() * theta.sin(), mu.cosh() * theta.cos());
        let p = p1.inv();
        let z = Complex64::new(0.0, 1.0 / theta.cos());

        // s^2 - (bw / r) s + w0^2 = 0 for each LP root r.
        for (root, out) in [(p, &mut poles), (z, &mut zeros)] {
            let b = Complex64::new(bw, 0.0) / root;
            let disc = (b * b - 4.0 * w0sq).sqrt();
            let r1 = (b + disc) * 0.5;
            let r2 = (b - disc) * 0.5;
            // Keep the upper-half-plane representative of each conjugate pair.
            out[2 * i] = if r1.im >= 0.0 { r1 } else { r1.conj() };
            out[2 * i + 1] = if r2.im >= 0.0 { r2 } else { r2.conj() };
        }
    }

    // Pair each pole with the zero nearest in frequency (sorted by imaginary part).
    poles.sort_by(|a, b| a.im.partial_cmp(&b.im).unwrap_or(std::cmp::Ordering::Equal));
    zeros.sort_by(|a, b| a.im.partial_cmp(&b.im).unwrap_or(std::cmp::Ordering::Equal));

    let bilinear = |s: Complex64| (Complex64::new(k, 0.0) + s) / (Complex64::new(k, 0.0) - s);

    let mut out = [Section::PASS; N];
    for i in 0..N {
        let pz = bilinear(poles[i]);
        let zz = bilinear(zeros[i]);
        // (z - q)(z - q*) = z^2 - 2 Re(q) z + |q|^2, written in z^-1 form.
        let (mut b0, mut b1, mut b2) = (1.0, -2.0 * zz.re, zz.norm_sqr());
        let (a1, a2) = (-2.0 * pz.re, pz.norm_sqr());
        // Normalise each section to unity gain at DC (z = 1).
        let g = (1.0 + a1 + a2) / (b0 + b1 + b2);
        b0 *= g;
        b1 *= g;
        b2 *= g;
        out[i] = Section { b0, b1, b2, a1, a2, z1: 0.0, z2: 0.0 };
    }
    out
}

#[cfg(test)]
mod notch_tests {
    use super::*;

    /// The notch must cut at least NOTCH_STOPBAND_DB across the whole labelled
    /// width, stay flat well outside it, and never boost.
    #[test]
    fn notch_covers_labelled_width() {
        for &fs in &[44100.0f32, 48000.0] {
            for &fc in &[500.0f32, 1000.0, 4000.0, 6000.0, 10000.0] {
                for &w in &[0.25f32, 0.5, 1.0] {
                    let mut n = BandReject::new();
                    n.set_notch(fs, fc, w);
                    let half = 2f32.powf(w / 2.0);
                    let (lo, hi) = (fc / half, fc * half);
                    for i in 0..=100 {
                        let f = lo * (hi / lo).powf(i as f32 / 100.0);
                        let db = n.response_db(fs, f);
                        assert!(db <= -NOTCH_STOPBAND_DB + 0.5,
                            "fs {fs} fc {fc} w {w}: only {db:.1} dB at {f:.0} Hz inside the band");
                    }
                    // 0.15 octave outside each edge: within 3 dB of unity.
                    for f in [lo / 2f32.powf(0.15), hi * 2f32.powf(0.15)] {
                        if f < fs * 0.45 {
                            let db = n.response_db(fs, f);
                            assert!(db > -3.0, "fs {fs} fc {fc} w {w}: {db:.1} dB at {f:.0} Hz outside");
                        }
                    }
                    for f in [20.0f32, 100.0, fc / 4.0, (fc * 4.0).min(fs * 0.45)] {
                        let db = n.response_db(fs, f);
                        assert!(db < 0.01, "boost of {db:.2} dB at {f:.0} Hz");
                    }
                }
            }
        }
    }

    /// Running noise through the filter stays finite and bounded (stability).
    #[test]
    fn notch_is_stable() {
        let mut n = BandReject::new();
        n.set_notch(48000.0, 6000.0, 0.25);
        let mut x: u32 = 12345;
        for _ in 0..480_000 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            let white = (x >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0;
            let y = n.process(white);
            assert!(y.is_finite() && y.abs() < 10.0);
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
