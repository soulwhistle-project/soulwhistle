# Neural Coherence Frequency Decoder

A Python tool for analyzing Neural Coherence audio files to extract binaural beat frequencies, detect sections, and reverse-engineer the frequency patterns used in Focus 10, Focus 12, and other consciousness exploration programs.

## Overview

This tool analyzes stereo audio files (FLAC, OGG, MP3, WAV) to:

- **Detect sections automatically** based on frequency changes (not hardcoded)
- **Extract carrier frequencies** from left and right channels
- **Calculate binaural beats** (the difference frequency your brain perceives)
- **Classify brainwave states** (Delta, Theta, Alpha, Beta, Gamma)
- **Map frequency timelines** showing how patterns evolve throughout the track

## Background: How Neural Coherence Works

Neural Coherence (Hemispheric Synchronization) uses **binaural beats** to influence brainwave activity:

1. **Two different frequencies** are played in each ear (e.g., 170 Hz left, 174 Hz right)
2. **Your brain perceives a third frequency** that is the difference (174 - 170 = 4 Hz)
3. **Brain entrainment occurs** as your brainwaves synchronize to this perceived frequency

### Brainwave Frequency Ranges

| Range | Frequency | State |
|-------|-----------|-------|
| **Delta** | 0.5-4 Hz | Deep sleep, unconscious |
| **Theta** | 4-8 Hz | Deep meditation, light sleep, hypnagogic state |
| **Alpha** | 8-12 Hz | Relaxed, awake, light meditation |
| **Beta** | 12-30 Hz | Normal waking consciousness, alert |
| **Gamma** | 30+ Hz | High-level information processing |

### Focus Levels

Monroe Institute programs use numbered "Focus" levels:

- **Focus 10**: "Mind awake, body asleep" - typically theta/alpha states
- **Focus 12**: "Expanded awareness" - beyond physical body perception
- **Focus 15, 21, 27**: Progressively deeper non-physical states

The exact proprietary frequencies are not publicly documented - this tool helps decode them.

## Installation

### Requirements

```bash
# System dependencies
sudo apt install ffmpeg  # For audio conversion

# Python dependencies (already available)
python3 -c "import numpy, scipy; print('Ready!')"
```

The script uses:
- `numpy` - numerical processing
- `scipy` - signal processing and FFT analysis
- `ffmpeg` - audio format conversion

## Usage

### Basic Analysis

```bash
# Analyze a single file
python3 coherence_analyzer.py /path/to/focus10.flac

# Save detailed JSON report
python3 coherence_analyzer.py /path/to/focus10.flac -o focus10_report.json
```

### Advanced Options

```bash
# Adjust temporal resolution
python3 coherence_analyzer.py audio.flac --chunk-duration 20 --overlap 10

# Analyze multiple files
for file in focus*.flac; do
    python3 coherence_analyzer.py "$file" -o "${file%.flac}_analysis.json"
done
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `audio_file` | (required) | Path to audio file (FLAC, OGG, MP3, WAV) |
| `-o, --output` | None | Save JSON report to file |
| `-c, --chunk-duration` | 30 | Analysis chunk size in seconds |
| `--overlap` | 15 | Overlap between chunks in seconds |

## How It Works

### 1. Section Detection

The tool analyzes the audio in two passes:

**First Pass**: Coarse frequency analysis
- 10-second windows with 5-second hops
- Extracts dominant frequencies from each channel
- Calculates binaural beat for each window

**Change Point Detection**:
- Identifies when binaural beat changes by >2 Hz
- Identifies when carrier frequency changes by >20 Hz
- Filters out noise spikes (verifies sustained changes)
- Merges sections closer than 30 seconds

**Result**: Variable number of sections based on actual frequency changes

### 2. Frequency Extraction

For each section:

**FFT Analysis**:
- Applies Hann window to reduce spectral leakage
- Computes Fast Fourier Transform (FFT)
- Focuses on 50-1000 Hz range (typical carrier frequencies)
- Identifies dominant frequency and top 5 peaks

**Binaural Beat Calculation**:
```
Binaural Beat = |Left Channel Frequency - Right Channel Frequency|
```

### 3. Temporal Analysis

Within each section:
- Divides into overlapping chunks (default: 30s chunks, 15s overlap)
- Tracks frequency evolution over time
- Detects subtle variations within sections

## Output Format

### Console Report

```
================================================================================
HEMI-SYNC FREQUENCY ANALYSIS REPORT
================================================================================
File: focus10.flac
Duration: 2100.0 seconds (35.0 minutes)
Sample Rate: 44100 Hz
================================================================================

Detected 5 sections:
  intro                :    0.0s -   60.0s ( 60.0s) - Binaural: 1.50 Hz
  section_2_theta      :   60.0s -  480.0s (420.0s) - Binaural: 4.20 Hz
  section_3_theta      :  480.0s - 1680.0s (1200.0s) - Binaural: 5.80 Hz
  section_4_theta      : 1680.0s - 2040.0s (360.0s) - Binaural: 4.10 Hz
  outro                : 2040.0s - 2100.0s ( 60.0s) - Binaural: 1.20 Hz

SECTION_2_THETA SECTION
--------------------------------------------------------------------------------
Time Range: 60.0s - 480.0s
Duration: 420.0s

Average Frequencies:
  Left Channel:      172.50 Hz
  Right Channel:     176.70 Hz
  Binaural Beat:       4.20 Hz

Temporal Analysis (14 chunks):
      Time  Left Hz  Right Hz  Beat Hz     Type
  ----------------------------------------------------
      60.0   172.45    176.63     4.18    Theta
      75.0   172.51    176.71     4.20    Theta
      90.0   172.48    176.68     4.20    Theta
  ...
```

### JSON Output

```json
{
  "file": "focus10.flac",
  "duration": 2100.0,
  "sample_rate": 44100,
  "sections": [
    {
      "section_type": "section_2_theta",
      "start_time": 60.0,
      "end_time": 480.0,
      "chunks": [
        {
          "time": 60.0,
          "left_freq": 172.45,
          "right_freq": 176.63,
          "binaural_beat": 4.18,
          "wave_type": "Theta",
          "left_top_freqs": [[172.45, 8523.2], [344.9, 1240.5]],
          "right_top_freqs": [[176.63, 8612.1], [353.26, 1198.3]],
          "amplitude": 1524.8
        }
      ],
      "average": {
        "left_freq": 172.50,
        "right_freq": 176.70,
        "binaural_beat": 4.20
      }
    }
  ]
}
```

## Interpreting Results

### Section Types

Sections are automatically named based on detected characteristics:

- **`intro`**: First section with low amplitude (startup phase)
- **`outro`**: Last section with low amplitude (wind-down phase)
- **`section_N_theta`**: Main sections named by brainwave type
- **`section_N_delta`**: Deep delta wave sections
- **`section_N_alpha`**: Alpha state sections

### Carrier Frequencies

Typical carrier frequencies range from 100-300 Hz:
- These are the actual tones played in each ear
- Not audible as distinct pitches due to masking and mixing
- The difference between left/right creates the binaural beat

### Binaural Beat Patterns

Common patterns in Neural Coherence files:

**Focus 10 (Mind Awake, Body Asleep)**:
- Usually 4-6 Hz (Theta range)
- May start higher and gradually decrease
- Sustained theta throughout main section

**Focus 12 (Expanded Awareness)**:
- May use 5-8 Hz (upper Theta, lower Alpha)
- Can include frequency shifts between sections
- Often longer sustained sections

## Example Analysis Workflow

### 1. Quick Overview

```bash
# Get section breakdown
python3 coherence_analyzer.py focus10.flac | grep "section"
```

### 2. Detailed Analysis

```bash
# Full analysis with JSON export
python3 coherence_analyzer.py focus10.flac -o focus10.json

# Extract just the binaural beats
jq '.sections[].average.binaural_beat' focus10.json
```

### 3. Compare Multiple Files

```bash
# Analyze Focus 10 and 12
python3 coherence_analyzer.py focus10.flac -o f10.json
python3 coherence_analyzer.py focus12.flac -o f12.json

# Compare section counts
echo "Focus 10 sections: $(jq '.sections | length' f10.json)"
echo "Focus 12 sections: $(jq '.sections | length' f12.json)"
```

## Technical Details

### FFT Parameters

- **Window Function**: Hann window (reduces spectral leakage)
- **Sample Rate**: Preserved from source (typically 44100 Hz)
- **Frequency Range**: 50-1000 Hz (excludes sub-bass and high frequencies)
- **Peak Detection**: Finds frequencies with magnitude >30% of dominant peak

### Section Detection Algorithm

1. Extract RMS amplitude and dominant frequencies every 5 seconds
2. Apply Savitzky-Golay smoothing filter (removes noise while preserving edges)
3. Detect sustained changes in:
   - Binaural beat (>2 Hz difference)
   - Carrier frequency (>20 Hz difference)
4. Verify changes persist for 10+ seconds (filters spikes)
5. Merge sections separated by <30 seconds

### Accuracy Considerations

**Highly Accurate**:
- Binaural beat frequencies (±0.1 Hz)
- Section timing (±2-5 seconds)
- Dominant carrier frequencies (±1-2 Hz)

**Limitations**:
- Cannot detect frequencies below ~0.5 Hz (requires longer analysis windows)
- May miss very subtle frequency modulations (<1 Hz)
- Harmonic content might be detected instead of fundamental frequency

## Advanced Use Cases

### Creating Your Own Binaural Beats

Use the decoded frequencies to create custom tracks:

```python
import numpy as np
from scipy.io import wavfile

# Example: Create 4 Hz theta binaural beat
sample_rate = 44100
duration = 60  # seconds

# Carrier frequencies from analysis
left_freq = 172.5   # Hz
right_freq = 176.7  # Hz (difference = 4.2 Hz)

# Generate tones
t = np.linspace(0, duration, int(sample_rate * duration))
left = np.sin(2 * np.pi * left_freq * t)
right = np.sin(2 * np.pi * right_freq * t)

# Create stereo file
stereo = np.column_stack((left, right))
wavfile.write('custom_4hz_theta.wav', sample_rate, stereo.astype(np.float32))
```

### Batch Processing

```bash
#!/bin/bash
# Analyze all Neural Coherence files in a directory

mkdir -p analysis_results

for file in *.flac; do
    echo "Analyzing $file..."
    python3 coherence_analyzer.py "$file" \
        -o "analysis_results/${file%.flac}.json" \
        2>&1 | tee "analysis_results/${file%.flac}.log"
done

# Create summary
echo "=== ANALYSIS SUMMARY ===" > analysis_results/SUMMARY.txt
for json in analysis_results/*.json; do
    name=$(basename "$json" .json)
    sections=$(jq '.sections | length' "$json")
    duration=$(jq '.duration / 60' "$json")
    echo "$name: $sections sections, ${duration} minutes" >> analysis_results/SUMMARY.txt
done
```

## Troubleshooting

### FFmpeg Not Found

```bash
sudo apt install ffmpeg
# or on macOS:
brew install ffmpeg
```

### "Audio file must be stereo"

Neural Coherence files require separate left/right channels. If you have a mono file:

```bash
# Convert mono to stereo (duplicate channel)
ffmpeg -i mono.flac -ac 2 stereo.flac
```

### Memory Issues with Large Files

For very long files (>2 hours):

```bash
# Use larger chunks, less overlap
python3 coherence_analyzer.py long_file.flac --chunk-duration 60 --overlap 10
```

## References

### Neural Coherence Technology

- **Monroe Institute**: https://www.monroeinstitute.org/
- **Wikipedia - Neural Coherence**: https://en.wikipedia.org/wiki/Robert_Monroe#Neural Coherence
- **Binaural Beats**: https://en.wikipedia.org/wiki/Binaural_beats

### Scientific Background

- Monroe, R. A. (1971). *Journeys Out of the Body*. Doubleday.
- Oster, G. (1973). "Auditory beats in the brain". *Scientific American*, 229(4), 94-102.
- CIA Gateway Process Report (1983) - Analysis of Monroe's techniques

### Audio Analysis

- FFT (Fast Fourier Transform): https://en.wikipedia.org/wiki/Fast_Fourier_transform
- Spectral Analysis: https://docs.scipy.org/doc/scipy/reference/signal.html

## Legal & Ethical Notes

**This tool is for research and educational purposes:**

- ✅ Analyzing files you own for personal understanding
- ✅ Academic research on consciousness and brainwave entrainment
- ✅ Creating your own binaural beat compositions
- ❌ Pirating or distributing copyrighted Monroe Institute content
- ❌ Commercial use of proprietary frequency patterns without permission

**Respect intellectual property**: The Monroe Institute's specific frequency sequences, narration, and guidance are copyrighted. This tool helps understand the underlying technology, not replicate commercial products.

## Future Enhancements

Potential improvements to this tool:

- [ ] Detect frequency modulation (FM) and amplitude modulation (AM)
- [ ] Identify pink noise, ocean sounds, and other background elements
- [ ] Generate visualization plots (spectrograms, frequency timelines)
- [ ] Detect voice/guidance sections vs pure tones
- [ ] Compare multiple files to find pattern similarities
- [ ] Export findings to CSV for spreadsheet analysis

## Contributing

Found a bug or have an enhancement idea? This tool is part of the uapwhistle repository.

## License

See the main repository LICENSE file. This tool is provided as-is for research purposes.

---

**Note**: The exact frequencies used in commercial Neural Coherence products are proprietary to The Monroe Institute. This tool performs independent analysis and does not use or reference any confidential information.
