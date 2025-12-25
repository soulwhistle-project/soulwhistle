# SoulWhistle - Multi-Dimensional Communication Research Tool

A real-time audio synthesizer for consciousness research and exploration, designed for researchers, consciousness explorers, and those seeking to advance our understanding of human potential and multi-dimensional communication.

## Purpose

SoulWhistle is a research instrument that combines scientifically decoded Neural Coherence binaural frequencies with experimental signal generation, enabling:

- **Consciousness Research**: Explore altered states using verified Monroe Institute Gateway Experience frequencies
- **Self-Discovery**: Investigate your own consciousness through precise brainwave entrainment
- **Signal Analysis**: Generate and study various frequency combinations and their effects
- **Experimental Contact**: Test hypotheses about communication across different states of consciousness

This tool empowers researchers and explorers to conduct reproducible experiments with the same frequencies used in decades of Monroe Institute research, while providing the flexibility to design custom protocols.

## Dual-Path Architecture

SoulWhistle implements **separate signal paths** for acoustic (speaker) and electromagnetic (RF) output:

- **Acoustic Path (Speakers)**: Uses psychoacoustic tricks (100 Hz AM) to approximate 7.83 Hz Schumann resonance for human consciousness work
- **RF Path (HackRF)**: Transmits TRUE 7.83 Hz envelope modulation and full ultrasonic bandwidth for electromagnetic experiments

See [DUAL_PATH_ARCHITECTURE.md](DUAL_PATH_ARCHITECTURE.md) for technical details on how this overcomes speaker physics limitations.

## Features

### Dual-Mode Operation

SoulWhistle operates in two distinct modes for different research objectives:

#### 1. Signal Layer Mode
Experimental frequency generation for signal research and exploration:
   - 7.83 Hz Schumann carrier with 100 Hz modulation
   - 528 Hz healing harmonic
   - 17 kHz ultrasonic pings
   - Organic chirps at 2.5 kHz
   - 432 Hz ambient pad
   - Breathing layer with LFO modulation

#### 2. Human Consciousness Modes
Scientifically decoded Monroe Institute frequencies for consciousness research:

**Focus 10** - "Mind Awake, Body Asleep"
- Decoded from Monroe Institute Gateway Experience tapes
- 4.11 Hz theta binaural beat (100.12 Hz left / 104.23 Hz right)
- Includes 220 Hz harmonic cluster (as in original tapes)
- Optimized for meditation and out-of-body exploration

**Focus 12** - "Expanded Awareness"
- Decoded from Gateway Experience Wave IV tapes
- 1.5 Hz deep delta binaural beat (100.77 Hz left / 99.27 Hz right)
- Includes 495 Hz high harmonic for "expansion" feeling
- Ultra-stable frequency for deep consciousness exploration

**Deep Focus - ADHD Suite** - Research-backed frequencies for sustained attention (4 presets)
- **Calm (10 Hz Alpha)**: Reading, creative work, relaxed focus
- **ADHD (14 Hz SMR)**: Primary ADHD support, sustained attention (most researched)
- **Active (18 Hz Beta)**: Analytical tasks, coding, problem-solving
- **Peak (40 Hz Gamma)**: Intensive short bursts, cognitive binding (use with caution)
- See [DEEP_FOCUS_ADHD_GUIDE.md](DEEP_FOCUS_ADHD_GUIDE.md) for comprehensive usage guide

**Custom Mode** - User-defined binaural frequencies for experimental research
- Set any frequency from 0.1 to 30 Hz
- Delta (0-4 Hz), Theta (4-8 Hz), Alpha (8-12 Hz), Beta (12-30 Hz)
- Automatic harmonic enablement based on frequency range
- Design your own consciousness exploration protocols

## Controls

### Navigation
- **Up/Down** or **j/k**: Select channel
- **Left/Right** or **h**: Adjust volume/parameters
- **Space**: Play/Pause (or toggle RF on RF line)
- **o**: Cycle oscillator/modulation type for selected channel
- **m**: Mute/unmute selected channel
- **s**: Save current settings as preset
- **l**: Load preset from list
- **q**: Quit

### ⚠️ Important: Headphones Required for Human Modes
**Binaural beats ONLY work with stereo headphones!** Each ear must receive a different frequency for your brain to create the perceived "beat" frequency. Using speakers mixes the frequencies in the air before reaching your ears, preventing the binaural effect.

- **uap-dogwhistle mode**: Speakers OK (no binaural beats)
- **human-focus10/12/custom modes**: 🎧 **Headphones REQUIRED** for consciousness effects

### Mode Selection
1. Navigate to "Preset / Mode"
2. Use **Left/Right** arrows to cycle through modes:
   - **uap-dogwhistle**: Signal layer only, no binaural beats (speakers OK)
   - **human-focus10**: 4.1Hz theta binaural (🎧 headphones required)
   - **human-focus12**: 1.5Hz delta binaural (🎧 headphones required)
   - **human-custom**: Custom Hz binaural + signal layer (🎧 headphones required)
3. Preset frequencies auto-apply when you select Focus 10/12
4. In human modes, adjust Neural Coherence volume to control binaural beat intensity
5. For Custom mode, adjust binaural beat frequency using arrow keys to design your own protocol

### Preset System

Built-in research presets in `presets/` directory:
- `DEFAULT_focus_10_mind_awake.json` - Monroe Institute Focus 10 frequencies (immutable)
- `DEFAULT_focus_12_expanded.json` - Monroe Institute Focus 12 frequencies (immutable)
- `DEFAULT_signal_layer.json` - Experimental signal frequencies (immutable)
- Save your own custom configurations with the **s** key
- Load presets with the **l** key for reproducible research sessions

## Technical Details

### Neural Coherence Implementation

Based on frequency analysis of actual Gateway Experience tapes:

**Focus 10 (GE1_Discovery_6_Free_Flow_10.flac):**
- Left carrier: 100.12 Hz
- Right carrier: 104.23 Hz
- Binaural beat: 4.11 Hz (theta state)
- Secondary harmonic: 220 Hz cluster
- Duration: 33.6 minutes in original

**Focus 12 (GE4_Adventure_3_Free_Flow_12.flac):**
- Left carrier: 100.77 Hz
- Right carrier: 99.27 Hz  
- Binaural beat: 1.5 Hz (deep delta)
- High harmonic: 495 Hz (for expansion)
- Duration: 35.1 minutes in original

### Brainwave States

- **Delta (0.5-4 Hz)**: Deep sleep, unconscious, healing
- **Theta (4-8 Hz)**: Deep meditation, hypnagogic state, "body asleep"
- **Alpha (8-12 Hz)**: Relaxed, light meditation, creative flow
- **Beta (12-30 Hz)**: Normal waking consciousness, alert thinking
- **Gamma (30+ Hz)**: High-level processing, peak concentration

## HackRF Integration

RF transmission available for experimental research purposes.

### ⚠️ CRITICAL SAFETY WARNING ⚠️

**READ THIS BEFORE ENABLING RF TRANSMISSION:**

- **NEVER operate RF transmission during flight** - Can interfere with aircraft navigation and communication systems
- **Can interfere with GPS** - May disrupt GPS receivers in your vicinity
- **Can interfere with critical infrastructure** - Depending on frequency, may affect emergency services, aviation, maritime, military, or commercial communications
- **Requires proper licensing** - You MUST hold appropriate amateur radio or experimental licenses for the frequencies you transmit on
- **Legal compliance is YOUR responsibility** - Ensure compliance with FCC regulations (US) or equivalent regulatory bodies in your jurisdiction
- **Illegal transmissions carry severe penalties** - Fines and criminal prosecution are possible for unlicensed or harmful interference
- **Know your local regulations** - Frequency allocations, power limits, and licensing requirements vary by country
- **Default frequency (1.42 GHz)** - This is the protected Hydrogen Line used for radio astronomy. Transmission on this frequency may be ILLEGAL in your area

### Technical Specifications
- Default: 1.42 GHz (Hydrogen Line - **CHECK LOCAL REGULATIONS**)
- Modes: WBFM, NBFM, AM
- Adjustable gain: 0-47 dB
- **RF transmission is DISABLED by default for safety**

### Before You Enable RF:
1. Verify you have the legal right to transmit on your chosen frequency
2. Ensure you are not near airports, hospitals, or other critical infrastructure
3. Check that you are not interfering with protected frequency bands
4. Understand the potential range and impact of your transmission
5. Have emergency shutdown procedures ready

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/soulwhistle-project/soulwhistle
cd soulwhistle

# Build
cargo build --release

# The binary will be at: target/release/soulwhistle
```

### First Run Setup

The application will automatically create a `presets/` folder on first run. For the best experience, copy the DEFAULT preset files:

```bash
# If you cloned the repo, presets are already included
# If you only have the binary, create presets folder:
mkdir -p presets

# Copy or download the DEFAULT_ preset files to presets/
# These provide the Monroe Institute Focus 10/12 frequencies
```

### Running

```bash
# From source:
cargo run --release

# Or run the binary directly:
./target/release/soulwhistle
```

## Research Background

The Neural Coherence frequencies were reverse-engineered from Monroe Institute Gateway Experience audio files using FFT spectral analysis. See `COMPLETE_FREQUENCY_MAP.md` for complete analysis.

Key discoveries:
- Focus 10 uses 4.1 Hz theta + gamma interrupts to prevent sleep
- Focus 12 uses ultra-stable 1.5 Hz delta for 18+ minutes
- Both use multi-frequency layering (carrier + harmonics)
- Harmonic clusters at ~220 Hz and ~495 Hz add texture without disrupting entrainment

### Dual-Path Signal Architecture

SoulWhistle implements separate signal processing for acoustic (speaker) and electromagnetic (RF) output:
- **Speakers**: Use psychoacoustic approximation (100 Hz AM) for 7.83 Hz Schumann resonance
- **RF Transmission**: TRUE 7.83 Hz envelope modulation on RF carrier (not psychoacoustic trick)
- See `DUAL_PATH_ARCHITECTURE.md` for complete technical details

## License

Experimental research tool. Neural Coherence preset frequencies decoded from publicly available Monroe Institute materials for educational purposes.

## Disclaimers

### Audio Safety
- **Use headphones** for proper binaural beat effect (required for brainwave entrainment)
- **Start with low volume** and gradually increase to comfortable levels
- **Do not use while driving or operating machinery** - Binaural beats can induce altered states of consciousness
- **Not medical advice** - This is a research tool for consciousness exploration, not a medical device
- **Consult a physician** if you have epilepsy, seizure disorders, or other neurological conditions

### RF Transmission Legal Disclaimer
- **You are solely responsible** for ensuring legal compliance with all RF transmission regulations
- **This software provides NO guarantees** of regulatory compliance
- **The developers assume NO liability** for illegal transmissions, interference, or damages caused by use of RF features
- **By enabling RF transmission, you accept full legal responsibility** for your actions
- **When in doubt, keep RF disabled** - The audio features alone provide full research functionality

### Research Use
- This tool is for **educational and research purposes only**
- Results are subjective and not scientifically validated for any specific outcome
- Individual experiences with consciousness exploration vary widely
- **Document your research** - Keep detailed notes of settings, experiences, and observations

---

## Documentation Index

### Quick Start
- [README.md](../README.md) - Main project overview
- [COMPLETE_FREQUENCY_MAP.md](COMPLETE_FREQUENCY_MAP.md) - **Master frequency reference** for all Focus levels
- [DEEP_FOCUS_ADHD_GUIDE.md](DEEP_FOCUS_ADHD_GUIDE.md) - Comprehensive ADHD preset usage guide

### Technical Documentation
- [DUAL_PATH_ARCHITECTURE.md](DUAL_PATH_ARCHITECTURE.md) - Acoustic vs RF signal processing
- [DATA_INTEGRATION_GUIDE.md](DATA_INTEGRATION_GUIDE.md) - Understanding JSON analysis files
- [COHERENCE_DECODE.md](COHERENCE_DECODE.md) - FFT analysis tool documentation
- [ANALYZER_VALIDATION.md](ANALYZER_VALIDATION.md) - Methodology validation
- [TESTING.md](TESTING.md) - QA and testing procedures
- [CHANGELOG.md](CHANGELOG.md) - Version history

### Research & Analysis
- [FOCUS_10_ANALYSIS.md](../research/FOCUS_10_ANALYSIS.md) - Complete Focus 10 breakdown
- [FOCUS_15_ANALYSIS.md](../research/FOCUS_15_ANALYSIS.md) - Complete Focus 15 breakdown
- [FOCUS_21_ANALYSIS.md](../research/FOCUS_21_ANALYSIS.md) - Complete Focus 21 breakdown (OBE state)
- [MONROE_INSTITUTE_RESEARCH.md](../research/MONROE_INSTITUTE_RESEARCH.md) - Historical context & peer review
- [ALPHA_THETA_RESEARCH_SUMMARY.md](../research/ALPHA_THETA_RESEARCH_SUMMARY.md) - Binaural beat science
- [FUTURE_IDEAS.md](../research/FUTURE_IDEAS.md) - Speculative research directions

### Citations & Data
- [CITATIONS.md](../CITATIONS.md) - **Complete bibliography** (21 peer-reviewed sources)
- [data/README.md](../data/README.md) - Preset file documentation
- `/data/*.json` - FFT analysis files (see [DATA_INTEGRATION_GUIDE.md](DATA_INTEGRATION_GUIDE.md))

### Navigation Tips
- For **frequency details**: Start with [COMPLETE_FREQUENCY_MAP.md](COMPLETE_FREQUENCY_MAP.md)
- For **research citations**: See [CITATIONS.md](../CITATIONS.md)
- For **ADHD support**: Read [DEEP_FOCUS_ADHD_GUIDE.md](DEEP_FOCUS_ADHD_GUIDE.md)
- For **data analysis**: Use [DATA_INTEGRATION_GUIDE.md](DATA_INTEGRATION_GUIDE.md)
- For **OBE practice**: Focus 21 details in [FOCUS_21_ANALYSIS.md](../research/FOCUS_21_ANALYSIS.md)
