# Soulwhistle TODO & Roadmap

## Piezoelectric Transducer Support (Planned)

A primary goal is to support piezoelectric transducers for ultrasonic audio research above 20 kHz. This requires:
- High sample rate audio interfaces (96 kHz minimum, 192 kHz preferred)
- Sample rate detection and Nyquist limit warnings in UI
- Device selection for multi-interface systems
- Real-time validation when frequencies exceed audio capabilities

**Hardware Requirements:**
- USB audio interface with 96+ kHz sample rate
- Piezoelectric transducers rated for target frequencies

**Audio Nyquist Limits (maximum representable frequency = sample_rate ÷ 2):**
- 48 kHz sample rate → 24 kHz maximum frequency
- 96 kHz sample rate → 48 kHz maximum frequency
- 192 kHz sample rate → 96 kHz maximum frequency

**Current Status:**
- ✅ Frequency range supports up to 100 kHz (software ready)
- ⚠️ No sample rate override or audio device selection (the synth always runs at the default device's rate; this is the main blocker)
- ✅ The ping layer is muted above 0.45 × sample rate and the UI says so (since 0.3.0)
- ⚠️ No warning yet for other layers or for presets loaded with out-of-range values

**Pull requests for piezo transducer integration are highly encouraged!**

## True 40 Hz Auditory Stimulation

Done: the ping layer has a `GammaPulse` type, and the "Gamma 40 Hz Pulses" preset reproduces the Martorell et al. 2019 stimulus (1 ms tones at 10 kHz, 40 per second, both ears). Still open:

- Check the listening level used in Martorell 2019 (secondary sources say 60 dB) and Chan et al. 2022 (40 Hz clicks) against the papers' methods, then add both to the preset text if confirmed.
- Optional soft (raised-cosine) edges for the bursts, as a comfort setting.

## Network Streaming (Improvements)

- ✅ **FLAC endpoint** (`/stream.flac`, `?fmt=flac16`): done. Checked with the reference `flac` decoder (sample-exact test), ffprobe, mpv and VLC on Android. Still to check: desktop VLC, Chrome and Firefox. Possible next step: live title updates after a preset change (tags are only read on connect).
- **Clock drift**: the laptop's and the listener's sound cards run at slightly different speeds, so over many hours a slower listener falls 2 s behind and skips ahead with a click. Fixing it needs adaptive resampling on the reader side.
- **Sample rate choice**: ultrasonic layers only stream if the device runs at 96 or 192 kHz (see piezo section above).

## OpenBCI Integration (Planned)

A primary goal is to integrate with OpenBCI hardware for closed-loop brainwave entrainment research. This would enable:
- Real-time EEG monitoring during sessions
- Automatic frequency adjustment based on brain state
- Data correlation between stimulus and response
- Reproducible experimental protocols

**Pull requests for OpenBCI integration are highly encouraged!**
