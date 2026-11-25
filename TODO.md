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

## True 40 Hz Auditory Stimulation (Planned)

The 40 Hz preset is a binaural beat, but the 40 Hz studies used pulsed sound, not binaural beats (see `CITATIONS.md`, entries 22 and 30 to 34). Add a pulsed mode that reproduces the published stimulus:

- **Martorell et al. 2019 (Cell)**: trains of 1 ms tones at 10 kHz, one every 25 ms (40 per second), from the paper's methods. Secondary sources give the level as 60 dB; check the STAR Methods before quoting it.
- **Chan et al. 2022 (PLOS ONE)**: 40 Hz click sound paired with 40 Hz light (described in secondary sources; check the paper's methods).
- Same signal in both ears (it is not a binaural effect), so it works on speakers as well as headphones.
- Implementation sketch: a new layer or `SignalType` generating a tone burst of N cycles at a carrier, repeated at a rate (defaults 10 kHz, 1 ms, 40 Hz), with an option for a short raised-cosine ramp instead of a hard gate. Sample-accurate timing from a sample counter, no allocation in the callback.
- Preset text must stay honest: the evidence is from mice and from light-and-sound devices, and Cognito's HOPE trial missed its primary endpoint.

## Network Streaming (Improvements)

- **FLAC endpoint** (`/stream.flac`): lossless at about half the bandwidth of WAV. The pure-Rust `flacenc` crate (0.5.1, Apache-2.0, builds on stable) encodes block by block with `encode_fixed_size_frame`, which fits a live stream: write the `fLaC` header and STREAMINFO with an unknown total length, then one frame per block (for example 4096 samples). Encode on the reader thread, never in the audio callback. FLAC carries integers only: 16 or 24 bit.
- **Clock drift**: the laptop's and the listener's sound cards run at slightly different speeds, so over many hours a slower listener falls 2 s behind and skips ahead with a click. Fixing it needs adaptive resampling on the reader side.
- **Sample rate choice**: ultrasonic layers only stream if the device runs at 96 or 192 kHz (see piezo section above).

## OpenBCI Integration (Planned)

A primary goal is to integrate with OpenBCI hardware for closed-loop brainwave entrainment research. This would enable:
- Real-time EEG monitoring during sessions
- Automatic frequency adjustment based on brain state
- Data correlation between stimulus and response
- Reproducible experimental protocols

**Pull requests for OpenBCI integration are highly encouraged!**
