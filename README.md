# Soulwhistle

**An open-source platform for consciousness research using precise audio and radio frequency generation.**

Soulwhistle is a real-time audio synthesis engine designed for researchers, practitioners, and developers working with brainwave entrainment, binaural beats, and consciousness exploration. Generate custom frequencies, design multi-phase sessions, and integrate with research hardware.

## Core Capabilities

### Audio Synthesis
- **Binaural Beats**: Any frequency from 0.1 to 30 Hz (delta, theta, alpha, beta, gamma)
- **Custom Carriers**: Configurable carrier frequencies (optimal range: 300-600 Hz)
- **Multi-Layer Mixing**: Schumann resonance (7.83 Hz), harmonics (528 Hz), ambient pads (432 Hz)
- **Signal Types**: Sine, triangle, square, sawtooth, FM/AM modulation, organic chirps
- **Session Phases**: Multi-phase sessions with independent beat frequencies per phase

### Research Features
- **Real-Time Control**: Live adjustment of frequencies, volumes, and waveforms via TUI
- **Network Streaming**: HTTP audio streaming for wireless playback (experimental)
- **RF Transmission**: HackRF One integration for radio frequency experiments
- **Preset System**: JSON-based presets for reproducible experiments

### Example Applications
- Brainwave entrainment studies
- Meditation and focus enhancement
- Sleep research (delta/theta transitions)
- Consciousness state exploration
- Custom therapeutic protocols

## Getting Started

### Installation

**Option 1: Install from crates.io (recommended)**

```bash
cargo install soulwhistle
soulwhistle
```

**Option 2: Build from source**

```bash
# Clone repository
git clone https://github.com/soulwhistle-project/soulwhistle
cd soulwhistle

# Build and install
cargo install --path .

# Or just build
cargo build --release
./target/release/soulwhistle
```

Presets are automatically installed to:
- Linux: `~/.config/soulwhistle/presets/`
- macOS: `~/Library/Application Support/soulwhistle/presets/`
- Windows: `%APPDATA%\soulwhistle\presets\`

### Quick Start

1. **Select a preset**: Use `p` key to browse included research presets
2. **Adjust parameters**: Navigate with arrow keys, adjust with left/right arrows
3. **Control playback**: Space to play/pause, `m` to toggle oscillator types
4. **Lock signal layer**: `x` to lock/unlock core frequencies during experimentation

### Basic Controls

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate channels |
| `←/→` | Adjust values |
| `Space` | Play/pause |
| `m` | Mute/unmute channel |
| `o` | Cycle oscillator type |
| `p` | Select preset |
| `x` | Lock/unlock signal layer |
| `c` | Collapse/expand sections |
| `q` | Quit |

## Research Presets

Soulwhistle includes presets based on peer-reviewed research and established protocols. Experimental presets (clearly marked with `[EXPERIMENTAL]` in the UI) are exploratory and not peer-reviewed.

### General Focus & Meditation
- **Deep Focus (Active)**: Beta/low-gamma for concentration (12-15 Hz)
- **Deep Focus (Calm)**: Alpha/theta for relaxed focus (8-10 Hz)
- **Deep Focus (ADHD)**: Optimized beta protocol for ADHD research
- **Deep Focus (Peak)**: High beta/gamma for peak performance (18-25 Hz)

### Monroe Institute-Derived Examples
These presets are based on independent FFT analysis of Monroe Institute techniques:
- **Focus 10**: "Mind awake, body asleep" state (alpha/theta transition)
- **Focus 12**: Expanded awareness state (theta with gamma bursts)
- **Focus 15**: "No-time" state (deep theta)
- **Focus 21**: Bridge state (complex multi-frequency)

### Animal Research
Ultrasonic and RF frequencies for behavioral research across multiple species (dogs 23-54 kHz, cats up to 79 kHz, rodents 30-110 kHz, marine mammals up to 160 kHz, bats 20-80 kHz).

**[➡️ Full peer-reviewed research in `research/ANIMAL_ULTRASONIC_RF_RESEARCH.md`](./research/ANIMAL_ULTRASONIC_RF_RESEARCH.md)**

### UAP Contact Research (Experimental)
**RESEARCH TOOLS ONLY - NO CLAIMS MADE**

These presets implement frequency combinations from anecdotal UAP research community reports. We provide these tools for independent experimentation only.

- **UAP Frequencies**: Multi-layered signal based on anecdotal reports
- **UAP Enigmatic Ideas**: Anecdotal implementation (100 Hz carrier AM @ 7.83 Hz)
- **UAP Brycehelm**: Bryce Helm's design (783 Hz carrier AM @ 7.83 Hz) - github.com/brycehelm/UAP_Dog_Whistle
- **UAP Sweep 18kHz**: UAPWatchers guide variant (requires piezo transducers or RF mode)
- **UAP Sweep 24kHz**: Extended ultrasonic sweep (requires RF transmission mode)
- **UAP RF Ultrasonic**: Anecdotal electromagnetic approach at 1.42 GHz Hydrogen Line (HackRF required)

**Important:**
- No scientific validation exists for UAP contact via audio/RF frequencies
- RF transmission requires appropriate licenses and legal authorization
- See `research/UAP_VARIANTS.md` for full documentation and legal warnings
- The term "dog whistle" in UAP communities refers to electromagnetic signaling devices, unrelated to animal research

### Other Experimental
These presets are exploratory and not based on peer-reviewed research:
- **Explore All**: Scans through all brainwave states sequentially

**[➡️ Full citation list in `CITATIONS.md`](./CITATIONS.md)**

## Contributing Research Presets

We welcome contributions of research-based binaural beat protocols!

### Preset Creator Skill

Soulwhistle includes a Claude Code skill (`.claude/skills/preset-creator/`) with:
- Peer-reviewed binaural beat research knowledge
- Optimal carrier frequency guidelines (300-600 Hz)
- Multi-phase session design patterns
- Safety guidelines and contraindications

### Contribution Requirements

All preset submissions must:
- ✅ Be grounded in peer-reviewed research OR clearly marked as experimental (`"experimental": true`)
- ✅ Include proper citations (research presets) or rationale (experimental presets) in the description
- ✅ Follow optimal carrier frequency guidelines
- ✅ Include safety warnings and contraindications
- ✅ Specify target use case and session duration

**[➡️ Full contribution guide in `CONTRIBUTING.md`](./CONTRIBUTING.md)**

## OpenBCI Integration (Planned)

A primary goal is to integrate with OpenBCI hardware for closed-loop brainwave entrainment research. This would enable:
- Real-time EEG monitoring during sessions
- Automatic frequency adjustment based on brain state
- Data correlation between stimulus and response
- Reproducible experimental protocols

**Pull requests for OpenBCI integration are highly encouraged!**

## Network Streaming (Experimental)

HTTP audio streaming allows wireless playback to VLC or other clients:

```bash
# In Soulwhistle UI
1. Navigate to NETWORK STREAMING section
2. Enable streaming (Space key)
3. Optionally adjust port (default: 1123)

# On client device
vlc http://<soulwhistle-ip>:1123/stream.wav
```

**Help Wanted**: This feature needs contributions for improved protocols, buffering, multi-client support, and documentation.

## Architecture

- **Rust**: High-performance real-time audio synthesis
- **CPAL**: Cross-platform audio output
- **TUI**: Real-time interactive mixer interface
- **HackRF**: Optional RF transmission support
- **JSON Presets**: Reproducible session configurations

## Legal & Safety

**For Educational & Research Purposes Only**

This is an independent, non-commercial research tool. It is not a medical device and makes no medical claims.

**Important Disclaimers:**
- Not intended to diagnose, treat, cure, or prevent any disease
- Consult a qualified healthcare professional for health concerns
- Use at your own risk
- Binaural beats may affect people differently; discontinue if you experience adverse effects

**Third-Party Research**: Some presets are derived from independent FFT analysis of publicly documented techniques (e.g., Monroe Institute). This project is not affiliated with any third-party organizations. Trademark references are used for factual description only (nominative fair use).

## Licensing

- **Source Code**: Apache License 2.0 (see `LICENSE`)
- **Documentation & Presets**: Creative Commons Attribution 4.0 International (CC BY 4.0)
- **Research Data**: CC BY 4.0 (attribution required)

## Citing Soulwhistle

If you use Soulwhistle in your research:

**APA Style:**
> Soulwhistle Project. (2025). *Soulwhistle: An open-source platform for consciousness research*. GitHub. https://github.com/soulwhistle-project/soulwhistle

**BibTeX:**
```bibtex
@misc{soulwhistle2025,
  author       = {{Soulwhistle Project}},
  title        = {Soulwhistle: An open-source platform for consciousness research},
  year         = {2025},
  publisher    = {GitHub},
  howpublished = {\url{https://github.com/soulwhistle-project/soulwhistle}}
}
```

## Project Vision

Create a fully-featured, open-source platform for consciousness research that:
- Enables reproducible experimental protocols
- Integrates with research hardware (OpenBCI, EEG devices)
- Provides a foundation for closed-loop brainwave entrainment
- Democratizes access to consciousness research tools
- Maintains rigorous scientific standards

## Contributing

Contributions welcome in:
- OpenBCI/EEG hardware integration
- Research preset development
- Network streaming improvements
- Documentation and guides
- Testing and validation

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## Collaboration & Community

**We welcome researchers, institutions, and organizations to collaborate with us.**

Whether you're:
- A consciousness researcher seeking to validate or improve our implementations
- From the Monroe Institute or similar organizations interested in collaboration
- An academic institution conducting brainwave entrainment studies
- A developer wanting to contribute code or integrate hardware
- Someone who has found inaccuracies in our research or code

**Please get in touch!** Open an issue, submit a pull request, or reach out directly at **soulwhistle@pm.me**. This project is about understanding consciousness together through open science and collaborative research. We value accuracy, scientific rigor, and welcome corrections, improvements, and partnerships.

Our goal is not competition but contribution to the broader field of consciousness research.

---

**Questions?** Open an issue on GitHub.
**Research?** See [CITATIONS.md](./CITATIONS.md) for all sources.
