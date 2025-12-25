# Future Ideas & Research

This document tracks potential features and research topics that are interesting but not on the immediate roadmap.

## Audio Compensation for Different Mediums (Gases and Liquids)

**Concept:**
The perceived pitch of a sound wave is dependent on the medium through which it travels. The software could theoretically compensate for this by pre-adjusting the output frequency based on the propagation medium.

The general formula is:
`output_frequency = target_perceived_frequency * (speed_of_sound_in_air / speed_of_sound_in_medium)`

---

### Gaseous Mediums (e.g., Helium)

The speed of sound is significantly different in gases other than air. For instance, sound travels approximately 2.8 times faster in helium.

**Example (Helium):**
- Speed of sound in air: ~343 m/s
- Speed of sound in helium: ~965 m/s
- To achieve a perceived target of **440 Hz** in a helium environment, the software would need to generate:
  `440 * (343 / 965) ≈ 156 Hz`

**Use Case:** Highly specialized scientific experiments. This is a very low-priority and potentially hazardous use case.

---

### Liquid Mediums (e.g., Water)

This has more practical applications, especially with specialized audio hardware like piezoelectric transducers or underwater speakers. Sound travels approximately 4.3 times faster in water than in air.

**Example (Water):**
- Speed of sound in air: ~343 m/s
- Speed of sound in water: ~1480 m/s
- To achieve a perceived target of **440 Hz** underwater, the software would need to generate:
  `440 * (343 / 1480) ≈ 102 Hz`

**Use Case & Devices:**
- **Marine Research:** Communicating with marine life or for acoustic studies.
- **Synchronized Swimming:** Providing music to athletes underwater.
- **Submarine/AUV Communication:** Transmitting signals via sonar or acoustic modems.
- **Hardware:** This would require connecting the system to devices like **piezoelectric transducers** or commercially available **underwater speakers**, which are designed to operate efficiently in a dense medium like water.

**Status:**
This is a theoretical feature for future research. While more practical than the helium use case, it still requires specialized, non-standard hardware and remains a low priority.

---

## Audio Playback Quality Improvements

**Concept:**
The current audio implementation uses the default CPAL audio backend with standard buffer sizes. This can lead to occasional glitches, chirps, or dropouts during playback, especially under system load.

### Potential Solutions:

#### 1. Buffer Size Optimization
- **Current State:** Using default CPAL buffer configuration
- **Improvement:** Implement configurable buffer sizes with larger buffers for smoother playback
- **Tradeoff:** Larger buffers = more latency but smoother audio; smaller buffers = lower latency but more prone to glitches
- **Implementation:** Add buffer size configuration to audio settings with presets (realtime/low-latency vs. smooth/high-latency)

#### 2. Professional Audio Server Integration
- **JACK Audio Connection Kit:** Real-time, low-latency audio server with priority scheduling
  - Pro: Excellent for real-time audio with consistent low latency
  - Con: Requires JACK daemon running, additional setup complexity

- **PipeWire (Pro-Audio Mode):** Modern replacement for PulseAudio with real-time capabilities
  - Pro: More modern, can replace PulseAudio/JACK, growing ecosystem
  - Con: Still maturing, system-dependent performance

- **ALSA Direct:** Bypass PulseAudio/PipeWire entirely for dedicated hardware access
  - Pro: Lowest possible latency, direct hardware control
  - Con: Exclusive device access, no mixing with other applications

#### 3. Proper Soundcard/DAC Support
- **External DACs:** Support for professional USB DACs with native sample rate matching
- **ASIO Support (Windows):** For Windows users with professional audio interfaces
- **CoreAudio Optimization (macOS):** Native low-latency audio on macOS

**Priority:** Medium - Affects user experience but current implementation is functional

**Status:** Research and experimentation needed to determine optimal approach for different use cases
