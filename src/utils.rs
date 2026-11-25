use crate::audio::SignalType;
use crate::constants::*;

/// Fast sine approximation using Bhaskara I's formula.
/// Max error < 0.001 — imperceptible for audio synthesis.
/// ~5-10x faster than f32::sin() (avoids libm call).
#[inline(always)]
pub fn fast_sin(x: f32) -> f32 {
    use std::f32::consts::PI;
    // Normalize x to [0, 2π)
    let mut x = x % (2.0 * PI);
    if x < 0.0 {
        x += 2.0 * PI;
    }

    // Map to [0, π] and track sign
    let (x, sign) = if x > PI {
        (x - PI, -1.0_f32)
    } else {
        (x, 1.0_f32)
    };

    // Bhaskara I: sin(x) ≈ 16x(π - x) / (5π² - 4x(π - x))
    let x_pi_minus_x = x * (PI - x);
    let pi_sq = PI * PI;
    sign * (16.0 * x_pi_minus_x) / (5.0 * pi_sq - 4.0 * x_pi_minus_x)
}

/// Soft clipping — passes signal clean below threshold, gently saturates above.
#[inline(always)]
pub fn soft_clip(x: f32) -> f32 {
    let abs_x = x.abs();
    if abs_x <= SOFT_CLIP_THRESHOLD {
        x
    } else {
        let over = (abs_x - SOFT_CLIP_THRESHOLD) * SOFT_CLIP_STEEPNESS;
        x.signum() * (SOFT_CLIP_THRESHOLD + SOFT_CLIP_KNEE * over.tanh())
    }
}

/// Number of harmonics of `freq_hz` that fit below `max_freq_hz` (at least 1).
#[inline(always)]
pub fn harmonics_below(freq_hz: f32, max_freq_hz: f32) -> usize {
    if freq_hz <= 0.0 {
        return 1;
    }
    ((max_freq_hz / freq_hz) as usize).max(1)
}

/// Generate a band-limited waveform sample for a given phase and signal type.
///
/// Square, saw and triangle are built by additive synthesis from their Fourier
/// series, keeping only the first `max_harmonic` harmonics (see `harmonics_below`).
/// Drawing them naively sample by sample puts harmonics above Nyquist, which fold
/// back as audible false tones: a 17 kHz square at 48 kHz produced a tone at
/// 3 kHz. `sin(k x)` and `cos(k x)` come from the Chebyshev recurrence, so each
/// sample costs two trig calls plus a multiply-add per harmonic.
///
/// Like any band-limited square or saw, the waveform overshoots by about 9% at the
/// edges (Gibbs phenomenon); the output soft clipper absorbs it.
pub fn generate_waveform(phase: f32, signal_type: SignalType, max_harmonic: usize) -> f32 {
    use std::f64::consts::PI;
    match signal_type {
        SignalType::Square | SignalType::Saw | SignalType::Triangle => {
            let (s1, c1) = (phase as f64).sin_cos();
            let two_c = 2.0 * c1;
            // sin(k x), cos(k x) for k = 1, advanced by the recurrence.
            let (mut s_prev, mut s_k) = (0.0f64, s1);
            let (mut c_prev, mut c_k) = (1.0f64, c1);
            let mut acc = 0.0f64;
            for k in 1..=max_harmonic {
                let kf = k as f64;
                match signal_type {
                    // sign(sin x) = 4/pi * sum over odd k of sin(kx)/k
                    SignalType::Square => if k % 2 == 1 { acc += s_k / kf },
                    // rising saw, 0 at x = 0: 2/pi * sum (-1)^(k+1) sin(kx)/k
                    SignalType::Saw => acc += if k % 2 == 1 { s_k / kf } else { -s_k / kf },
                    // triangle, -1 at x = 0 and +1 at x = pi: -8/pi^2 * sum odd cos(kx)/k^2
                    _ => if k % 2 == 1 { acc += c_k / (kf * kf) },
                }
                let s_next = two_c * s_k - s_prev;
                s_prev = s_k;
                s_k = s_next;
                let c_next = two_c * c_k - c_prev;
                c_prev = c_k;
                c_k = c_next;
            }
            let scale = match signal_type {
                SignalType::Square => 4.0 / PI,
                SignalType::Saw => 2.0 / PI,
                _ => -8.0 / (PI * PI),
            };
            (acc * scale) as f32
        }
        _ => fast_sin(phase), // Sine and default fallback
    }
}

/// Wrap text to a maximum width, optionally limiting number of lines
///
/// # Arguments
/// * `text` - The text to wrap
/// * `max_width` - Maximum characters per line
/// * `max_lines` - Optional maximum number of lines to return
/// * `indent` - Optional indentation string for each line
///
/// # Returns
/// Vector of wrapped lines
pub fn wrap_text(text: &str, max_width: usize, max_lines: Option<usize>, indent: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut line_count = 0;

    for word in text.split_whitespace() {
        // Check if we've reached max lines
        if let Some(max) = max_lines {
            if line_count >= max {
                break;
            }
        }

        // Check if adding this word would exceed max width
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                lines.push(format!("{}{}", indent, current_line));
                current_line = String::new();
                line_count += 1;
            }
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    // Add remaining text if within line limit
    if !current_line.is_empty() {
        if let Some(max) = max_lines {
            if line_count < max {
                lines.push(format!("{}{}", indent, current_line));
            }
        } else {
            lines.push(format!("{}{}", indent, current_line));
        }
    }

    lines
}

/// Apply waveform shaping to an audio signal value
pub fn apply_waveform_shaping(value: f32, signal_type: SignalType) -> f32 {
    match signal_type {
        SignalType::Square => {
            if value > 0.0 { 1.0 } else { -1.0 }
        },
        SignalType::Triangle => {
            // Use asin for shaping (input is already a signal value, not a phase)
            (value.clamp(-1.0, 1.0).asin() * 2.0 / std::f32::consts::PI)
                .clamp(-1.0, 1.0)
        },
        SignalType::Saw => {
            (value * 2.0 - 1.0).clamp(-1.0, 1.0)
        },
        _ => value,
    }
}

/// Cycle through a list with wrapping (for navigation)
/// Returns new index
pub fn cycle_index(current: usize, list_len: usize, direction: i32) -> usize {
    if list_len == 0 {
        return 0;
    }

    if direction > 0 {
        if current >= list_len - 1 { 0 } else { current + 1 }
    } else {
        if current == 0 { list_len - 1 } else { current - 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_basic() {
        let text = "This is a test of the text wrapping functionality";
        let lines = wrap_text(text, 20, None, "");
        assert!(lines.len() > 1);
        for line in lines {
            assert!(line.len() <= 20);
        }
    }

    #[test]
    fn test_wrap_text_with_max_lines() {
        let text = "Word1 Word2 Word3 Word4 Word5 Word6 Word7 Word8";
        let lines = wrap_text(text, 10, Some(2), "");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_wrap_text_with_indent() {
        let text = "Test text";
        let lines = wrap_text(text, 50, None, "  ");
        assert!(lines[0].starts_with("  "));
    }

    #[test]
    fn test_cycle_index_forward() {
        assert_eq!(cycle_index(0, 5, 1), 1);
        assert_eq!(cycle_index(4, 5, 1), 0); // Wrap around
    }

    #[test]
    fn test_cycle_index_backward() {
        assert_eq!(cycle_index(2, 5, -1), 1);
        assert_eq!(cycle_index(0, 5, -1), 4); // Wrap around
    }
}

#[cfg(test)]
mod waveform_tests {
    use super::*;
    use std::f32::consts::PI;

    /// Magnitude of one frequency in a signal (Goertzel), normalised so a unit
    /// sine at exactly that frequency gives 1.0.
    fn level(x: &[f32], fs: f32, f: f32) -> f64 {
        let w = 2.0 * std::f64::consts::PI * f as f64 / fs as f64;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &v in x {
            let s0 = v as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let p = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        2.0 * p.max(0.0).sqrt() / x.len() as f64
    }

    fn render(kind: SignalType, f: f32, fs: f32, naive: bool) -> Vec<f32> {
        let limit = fs * NYQUIST_SAFE_FRACTION;
        let mut phase = 0.0f32;
        (0..fs as usize)
            .map(|_| {
                let y = if naive {
                    let t = phase / (2.0 * PI);
                    match kind {
                        SignalType::Square => if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
                        SignalType::Saw => 2.0 * (t - (t + 0.5).floor()),
                        _ => 4.0 * (t - (t + 0.25).floor()).abs() - 1.0,
                    }
                } else {
                    generate_waveform(phase, kind, harmonics_below(f, limit))
                };
                phase = (phase + f / fs * 2.0 * PI) % (2.0 * PI);
                y
            })
            .collect()
    }

    /// Frequencies where the naive waveform's harmonics above Nyquist fold back.
    fn alias_freqs(f: f32, fs: f32) -> Vec<f32> {
        let nyq = fs / 2.0;
        (2..40)
            .map(|k| k as f32 * f)
            .filter(|&h| h > nyq)
            .map(|h| {
                let m = h % fs;
                if m > nyq { fs - m } else { m }
            })
            // Skip folds that land on a real harmonic or too close to DC.
            .filter(|&a| a > 50.0 && ((a / f).round() * f - a).abs() > 20.0)
            .collect()
    }

    #[test]
    fn band_limited_waveforms_have_no_audible_aliases() {
        let fs = 48000.0;
        for kind in [SignalType::Square, SignalType::Saw, SignalType::Triangle] {
            for f in [100.0f32, 432.0, 528.0, 2500.0, 6007.0, 9001.0, 17000.0] {
                let fundamental = level(&render(kind, f, fs, false), fs, f);
                let naive = render(kind, f, fs, true);
                let fixed = render(kind, f, fs, false);
                let mut worst_naive = -200.0f64;
                for a in alias_freqs(f, fs) {
                    let db = 20.0 * (level(&fixed, fs, a) / fundamental).log10();
                    assert!(db < -60.0, "{kind:?} {f} Hz: false tone at {a} Hz, {db:.1} dB");
                    worst_naive = worst_naive.max(20.0 * (level(&naive, fs, a) / fundamental).log10());
                }
                // The check must be able to see the problem it guards against.
                if f >= 2500.0 && kind != SignalType::Triangle {
                    assert!(worst_naive > -40.0, "{kind:?} {f} Hz: test did not detect naive aliasing ({worst_naive:.1} dB)");
                }
            }
        }
    }

    #[test]
    fn band_limited_shapes_match_the_naive_ones() {
        let h = 200; // plenty of harmonics for a shape check
        // Square: +1 in the first half cycle, -1 in the second.
        assert!((generate_waveform(PI / 2.0, SignalType::Square, h) - 1.0).abs() < 0.02);
        assert!((generate_waveform(3.0 * PI / 2.0, SignalType::Square, h) + 1.0).abs() < 0.02);
        // Saw: rising, 0 at 0, 0.5 at a quarter cycle.
        assert!(generate_waveform(0.0, SignalType::Saw, h).abs() < 0.02);
        assert!((generate_waveform(PI / 2.0, SignalType::Saw, h) - 0.5).abs() < 0.02);
        // Triangle: -1 at 0, +1 at half a cycle, 0 at a quarter.
        assert!((generate_waveform(0.0, SignalType::Triangle, h) + 1.0).abs() < 0.02);
        assert!((generate_waveform(PI, SignalType::Triangle, h) - 1.0).abs() < 0.02);
        assert!(generate_waveform(PI / 2.0, SignalType::Triangle, h).abs() < 0.02);
    }
}
