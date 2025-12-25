use crate::audio::SignalType;

/// Generate waveform sample for a given phase and signal type
pub fn generate_waveform(phase: f32, signal_type: SignalType) -> f32 {
    match signal_type {
        SignalType::Sine => phase.sin(),
        SignalType::Triangle => {
            // 2/pi * asin(sin(x))
            phase.sin().asin() * 2.0 / std::f32::consts::PI
        },
        SignalType::Square => {
            if phase.sin() >= 0.0 { 1.0 } else { -1.0 }
        },
        SignalType::Saw => {
            // 2 * (x/(2pi) - floor(x/2pi + 0.5))
            let x = phase / (2.0 * std::f32::consts::PI);
            2.0 * (x - (x + 0.5).floor())
        },
        _ => phase.sin(), // Default fallback
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
            (value.asin() * 2.0 / std::f32::consts::PI).clamp(-1.0, 1.0)
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
