# Data Integration Guide: Understanding the JSON Analysis Files

**Purpose:** This guide explains how to interpret and work with the FFT spectral analysis JSON files in the `/data/` directory.

**Target Audience:** Researchers, developers, and advanced users who want to understand the raw frequency data underlying SoulWhistle's presets.

---

## Overview

The `/data/` directory contains spectral analysis data extracted from Monroe Institute Gateway Experience audio programs using FFT (Fast Fourier Transform). These JSON files provide detailed frequency breakdowns that informed the creation of Focus presets.

### File Types

1. **FFT Analysis Files** (focus_*_analysis.json): Raw spectral analysis data from binaural beat frequency research
2. **Preset Files** (DEFAULT_*.json): User-facing preset configurations
3. **Custom Presets** (custom_*.json): User-created presets

---

## FFT Analysis File Structure

### File Naming Convention

```
focus_<level>_analysis.json

Examples:
- focus_10_analysis.json → Focus 10 (Mind Awake, Body Asleep)
- focus_12_analysis.json → Focus 12 (Expanded Awareness)
- focus_15_analysis.json → Focus 15 (No Time)
- focus_21_analysis.json → Focus 21 (Bridge to Other Realities - OBE)
```

### JSON Schema

```json
{
  "file": "Gateway Experience Wave I - Focus 10 Analysis",
  "source_attribution": "Monroe Institute Gateway Experience (analyzed for educational research)",
  "_legal_notice": "This file contains factual frequency data...",
  "duration": 2017.95,              // Total audio duration in seconds
  "sample_rate": 44100,              // Audio sample rate (Hz)
  "sections": [                      // Array of frequency sections
    {
      "section_type": "section_1_gamma",
      "start_time": 0.0,             // Section start (seconds)
      "end_time": 35.0,              // Section end (seconds)
      "chunks": [                    // Array of 30-second analysis windows
        {
          "time": 0.0,               // Chunk timestamp
          "left_freq": 119.53,       // Dominant left channel frequency (Hz)
          "right_freq": 238.03,      // Dominant right channel frequency (Hz)
          "binaural_beat": 118.5,    // Calculated beat frequency
          "wave_type": "Gamma",      // Brainwave classification
          "left_top_freqs": [        // Top 5 frequency peaks (left)
            [119.53, 5155119.77],    // [frequency_hz, amplitude]
            [159.9, 4324631.62],
            // ... 3 more peaks
          ],
          "right_top_freqs": [       // Top 5 frequency peaks (right)
            [238.03, 57850187.82],
            // ... 4 more peaks
          ],
          "amplitude": 1155.80       // Overall amplitude
        }
      ],
      "average": {                   // Section-level averages
        "left_freq": 119.53,
        "right_freq": 238.03,
        "binaural_beat": 118.5
      }
    }
  ]
}
```

---

## Key Fields Explained

### Header Level (Legal & Attribution)

| Field | Type | Description |
|-------|------|-------------|
| `file` | string | Generic description of analysis source (anonymized) |
| `source_attribution` | string | Attribution to Monroe Institute for research transparency |
| `_legal_notice` | string | Legal disclaimer stating these are factual data, not copyrighted content |
| `duration` | number | Total audio duration analyzed (seconds) |
| `sample_rate` | number | Audio sample rate used in analysis (Hz) |

**Legal Protection Note:** Original file paths have been removed to protect against copyright concerns. The frequency data itself (facts) is not copyrightable, but we maintain appropriate attribution to the Monroe Institute's research while ensuring no copyrighted material is distributed.

### Section Level

| Field | Type | Description |
|-------|------|-------------|
| `section_type` | string | Auto-generated label (e.g., "section_1_gamma", "section_4_theta") |
| `start_time` | number | Section start time in seconds |
| `end_time` | number | Section end time in seconds |
| `chunks` | array | 30-second analysis windows within this section |
| `average` | object | Average frequencies for the entire section |

### Chunk Level (30-second windows)

| Field | Type | Description |
|-------|------|-------------|
| `time` | number | Chunk timestamp (seconds from start) |
| `left_freq` | number | Dominant frequency in left channel (Hz) |
| `right_freq` | number | Dominant frequency in right channel (Hz) |
| `binaural_beat` | number | Difference between left and right (Hz) |
| `wave_type` | string | Brainwave classification: Delta, Theta, Alpha, Beta, Gamma |
| `left_top_freqs` | array | Top 5 frequency peaks in left channel (sorted by amplitude) |
| `right_top_freqs` | array | Top 5 frequency peaks in right channel |
| `amplitude` | number | Overall signal amplitude |

### Frequency Peak Format

Each entry in `left_top_freqs` and `right_top_freqs`:

```json
[frequency_hz, amplitude_value]

Example:
[119.53, 5155119.77]
  ↑           ↑
  Hz      Amplitude (arbitrary FFT units)
```

---

## Brainwave Classifications

The `wave_type` field categorizes binaural beat frequencies:

| Wave Type | Range (Hz) | Mental State |
|-----------|------------|--------------|
| **Delta** | 0.5 - 4.0 | Deep sleep, healing, unconscious |
| **Theta** | 4.0 - 8.0 | Meditation, hypnagogic, creativity |
| **Alpha** | 8.0 - 14.0 | Relaxation, light meditation |
| **Beta** | 14.0 - 30.0 | Alert, focused, analytical |
| **Gamma** | 30.0+ | High-level cognition, peak performance |

**Note:** Classifications are based on the binaural beat frequency, not the carrier frequencies.

---

## Understanding the Data

### Carrier Frequencies vs. Binaural Beats

**Carrier Frequencies** (`left_freq`, `right_freq`):
- The actual audio tones played in each ear
- Range: 50-500 Hz in Gateway recordings
- Example: 100.12 Hz (left) / 104.23 Hz (right)

**Binaural Beat** (`binaural_beat`):
- The perceived "phantom" frequency created in the brain
- Calculated as: `|left_freq - right_freq|`
- Example: |100.12 - 104.23| = 4.11 Hz

**The brain "hears" the 4.11 Hz beat, not the 100 Hz carriers!**

### Harmonic Content

The `left_top_freqs` and `right_top_freqs` arrays reveal harmonic complexity:

```json
"left_top_freqs": [
  [100.13, 5155119.77],   // Primary carrier (highest amplitude)
  [220.47, 4324631.62],   // 2nd harmonic (~220 Hz)
  [212.57, 4182911.32],   // Harmonic cluster
  [220.57, 4123347.92],   // Harmonic cluster
  [218.90, 3985060.40]    // Harmonic cluster
]
```

**Interpretation:**
- Primary frequency: 100 Hz carrier
- Secondary content: 220 Hz harmonic cluster
- Monroe's recordings use **multi-layered** approach, not simple sine waves

---

## Common Analysis Patterns

### Focus 10 (Mind Awake, Body Asleep)

```json
{
  "left_freq": 100.12,
  "right_freq": 104.23,
  "binaural_beat": 4.11,
  "wave_type": "Theta"
}
```

**Key Features:**
- Low carrier frequencies (~100 Hz)
- Mid-low theta binaural beat (4.11 Hz)
- 220 Hz harmonic clusters present
- Gamma bursts every 35 seconds (393 Hz beats)

### Focus 12 (Expanded Awareness)

```json
{
  "left_freq": 100.77,
  "right_freq": 99.27,
  "binaural_beat": 1.50,
  "wave_type": "Delta"
}
```

**Key Features:**
- Low carrier frequencies (~100 Hz)
- Deep delta binaural beat (1.50 Hz)
- Ultra-stable (±0.01 Hz variation)
- High harmonics (495-595 Hz) for "expansion" effect
- 18+ minutes continuous at 1.5 Hz

### Focus 15 (No Time)

```json
{
  "left_freq": 304.8,
  "right_freq": 300.0,
  "binaural_beat": 4.80,
  "wave_type": "Theta"
}
```

**Key Features:**
- **HIGH carrier frequencies (~300 Hz)** - 3x higher than F10/F12!
- Elevated theta binaural beat (4.80 Hz)
- Extreme gamma bursts (457 Hz)
- Fragmented sections (2-4 min each)

### Focus 21 (Bridge to Other Realities - OBE)

```json
{
  "left_freq": 200.0,
  "right_freq": 204.0,
  "binaural_beat": 4.00,
  "wave_type": "Theta"
}
```

**Key Features:**
- Balanced carrier frequencies (~200 Hz) - middle ground
- Theta/delta border binaural beat (4.00 Hz exactly)
- 15.7 minutes CONTINUOUS (longest section ever analyzed)
- NO gamma bursts (trusts mastery from previous levels)

---

## Data Processing Examples

### Python: Extract Binaural Beat Timeline

```python
import json

def extract_binaural_timeline(json_file):
    """Extract binaural beat frequency over time."""
    with open(json_file, 'r') as f:
        data = json.load(f)

    timeline = []
    for section in data['sections']:
        for chunk in section['chunks']:
            timeline.append({
                'time': chunk['time'],
                'binaural_beat': chunk['binaural_beat'],
                'wave_type': chunk['wave_type'],
                'left_carrier': chunk['left_freq'],
                'right_carrier': chunk['right_freq']
            })

    return timeline

# Usage
timeline = extract_binaural_timeline('data/focus_10_analysis.json')
for entry in timeline[:5]:
    print(f"{entry['time']}s: {entry['binaural_beat']} Hz ({entry['wave_type']})")
```

### Python: Find Main Focus State

```python
def find_main_state(json_file, min_duration=300):
    """Find longest continuous frequency section."""
    with open(json_file, 'r') as f:
        data = json.load(f)

    longest = None
    for section in data['sections']:
        duration = section['end_time'] - section['start_time']
        if duration >= min_duration:
            if longest is None or duration > longest['duration']:
                longest = {
                    'start': section['start_time'],
                    'end': section['end_time'],
                    'duration': duration,
                    'binaural_beat': section['average']['binaural_beat'],
                    'left_freq': section['average']['left_freq'],
                    'right_freq': section['average']['right_freq']
                }

    return longest

# Usage
main = find_main_state('data/focus_21_analysis.json')
print(f"Main state: {main['duration']/60:.1f} min at {main['binaural_beat']} Hz")
# Output: Main state: 15.7 min at 4.00 Hz
```

### Python: Visualize Frequency Progression

```python
import matplotlib.pyplot as plt

def plot_frequency_progression(json_file):
    """Plot binaural beat frequency over time."""
    timeline = extract_binaural_timeline(json_file)

    times = [entry['time'] for entry in timeline]
    beats = [entry['binaural_beat'] for entry in timeline]

    plt.figure(figsize=(12, 4))
    plt.plot(times, beats, marker='o', linestyle='-')
    plt.xlabel('Time (seconds)')
    plt.ylabel('Binaural Beat (Hz)')
    plt.title('Frequency Progression')
    plt.grid(True)
    plt.show()

# Usage
plot_frequency_progression('data/focus_10_analysis.json')
```

---

## Relationship to Presets

### How JSON Data Becomes Presets

1. **Analysis**: FFT analysis extracts frequency data → focus_*_analysis.json files
2. **Identification**: Longest continuous sections identified as "main states"
3. **Validation**: Frequencies verified across multiple chunks for consistency
4. **Preset Creation**: Average frequencies used to create DEFAULT_*.json presets

### Example: Focus 10 Preset Creation

**From JSON Analysis:**
```json
// data/focus_10_analysis.json
"section_4": {
  "start_time": 105.0,
  "end_time": 435.0,       // 330 seconds = 5.5 minutes
  "average": {
    "left_freq": 100.12,
    "right_freq": 104.23,
    "binaural_beat": 4.11
  }
}
```

**To Preset File:**
```json
// presets/DEFAULT_focus_10_mind_awake.json
{
  "name": "Focus 10 - Mind Awake, Body Asleep",
  "binaural_beat_hz": 4.11,
  "carriers": {
    "left_hz": 100.12,
    "right_hz": 104.23
  },
  "focus_level": 10
}
```

---

## Advanced Analysis Topics

### Detecting Gamma Bursts

Gamma bursts are brief high-frequency interruptions:

```python
def detect_gamma_bursts(json_file, threshold=100):
    """Find gamma burst patterns (>100 Hz binaural beats)."""
    timeline = extract_binaural_timeline(json_file)
    bursts = [entry for entry in timeline if entry['binaural_beat'] > threshold]
    return bursts

# Usage
bursts = detect_gamma_bursts('data/focus_10_analysis.json')
for burst in bursts:
    print(f"{burst['time']}s: {burst['binaural_beat']} Hz gamma burst")
```

### Frequency Stability Analysis

Measure frequency variation to assess stability:

```python
import statistics

def analyze_stability(json_file, section_index=0):
    """Analyze frequency stability within a section."""
    with open(json_file, 'r') as f:
        data = json.load(f)

    section = data['sections'][section_index]
    beats = [chunk['binaural_beat'] for chunk in section['chunks']]

    return {
        'mean': statistics.mean(beats),
        'stdev': statistics.stdev(beats),
        'min': min(beats),
        'max': max(beats),
        'variation': max(beats) - min(beats)
    }

# Usage
stability = analyze_stability('data/focus_12_analysis.json', section_index=4)
print(f"Focus 12 stability: ±{stability['variation']:.2f} Hz")
# Output: Focus 12 stability: ±0.01 Hz (ultra-stable!)
```

---

## File Inventory

### FFT Analysis Files

| File | Focus Level | Duration | Main Beat | Carriers |
|------|-------------|----------|-----------|----------|
| `focus_10_analysis.json` | F10 | 33.6 min | 4.11 Hz | 100.12/104.23 Hz |
| `focus_12_analysis.json` | F12 | 35.1 min | 1.50 Hz | 100.77/99.27 Hz |
| `focus_15_analysis.json` | F15 | 38.3 min | 4.80 Hz | 304.8/300.0 Hz |
| `focus_21_analysis.json` | F21 | 40.1 min | 4.00 Hz | 200.0/204.0 Hz |

### Preset Files (User-Facing)

Located in `/presets/`:
- `DEFAULT_focus_10_mind_awake.json`
- `DEFAULT_focus_12_expanded.json`
- `DEFAULT_focus_15_no_time.json`
- `DEFAULT_focus_21_bridge.json`
- `DEFAULT_deep_focus_calm.json` (10 Hz alpha)
- `DEFAULT_deep_focus_adhd.json` (14 Hz SMR)
- `DEFAULT_deep_focus_active.json` (18 Hz beta)
- `DEFAULT_deep_focus_peak.json` (40 Hz gamma)

---

## Quality Metrics

### Analysis Accuracy

- **Frequency Resolution:** ±0.05 Hz
- **Temporal Resolution:** 30-second windows with 5-second overlap
- **FFT Window Size:** Optimized for low-frequency detection
- **Validation Method:** Multiple independent analyses cross-verified

### Confidence Ratings

| Focus Level | Frequency Accuracy | Data Quality | Confidence |
|-------------|-------------------|--------------|------------|
| F10 | ±0.05 Hz | Clean peaks | ✅✅✅ HIGH |
| F12 | ±0.01 Hz | Ultra-stable | ✅✅✅ HIGH |
| F15 | ±0.10 Hz | Clear pattern | ✅✅✅ HIGH |
| F21 | ±0.05 Hz | Longest section | ✅✅✅ ABSOLUTE |

---

## Research Applications

### Using This Data for Research

1. **EEG Correlation Studies:**
   - Use JSON timestamps to sync with EEG recordings
   - Compare binaural beat frequencies to measured brainwaves
   - Test frequency-following response hypothesis

2. **Subjective Experience Mapping:**
   - Map JSON timeline to participant experience reports
   - Identify which sections correlate with specific phenomena
   - Validate Monroe's state progression theory

3. **Replication & Variation:**
   - Use frequencies as baseline for controlled experiments
   - Test carrier frequency importance (100 Hz vs 200 Hz vs 300 Hz)
   - Investigate gamma burst necessity

4. **Machine Learning:**
   - Train models to detect frequency patterns
   - Predict subjective effects from frequency profiles
   - Classify consciousness states from audio features

---

## Limitations & Considerations

### What This Data Captures

✅ Precise carrier frequencies
✅ Binaural beat frequencies
✅ Temporal structure
✅ Harmonic content
✅ Amplitude modulation

### What This Data Does NOT Capture

❌ Voice guidance content (removed from analysis)
❌ Background music/pink noise
❌ Spatial audio effects
❌ Subjective participant experiences
❌ Individual variation in response

---

## Further Reading

- `/docs/COMPLETE_FREQUENCY_MAP.md` - Master frequency reference
- `/research/FOCUS_*_ANALYSIS.md` - Detailed state-by-state breakdowns
- `/research/ANALYZER_VALIDATION.md` - Methodology validation
- `/research/MONROE_INSTITUTE_RESEARCH.md` - Historical context
- `CITATIONS.md` - Full bibliography

---

## Contributing

If you use this data in research, please:

1. Cite SoulWhistle project appropriately
2. Share findings with the community
3. Report any data quality issues
4. Contribute additional analysis scripts

---

**Data Generated:** December 23, 2025
**Analysis Tool:** coherence_analyzer.py (FFT-based)
**Format Version:** 1.0
**Status:** Complete for Focus 10, 12, 15, 21
