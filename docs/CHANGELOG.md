# Changelog

## Unreleased

- FLAC network stream: `/stream.flac` (24-bit) and `?fmt=flac16`, lossless at about half the bandwidth of WAV, encoded per listener with the pure-Rust `flacenc` crate, tagged with the preset title
- The streaming panel shows this machine's address and both stream links instead of `<ip>`
- Live title updates on the network stream (ICY metadata): players show the new preset's name after a preset change without reconnecting
- Clock correction on the network stream: each listener is resampled by at most 0.05 % to stay about 200 ms behind live, so long sessions no longer drift and skip (`?sync=off` for samples exactly as generated)
- `--list-devices`, `--device` and `--sample-rate`: choose the audio output and run the synth at 96 or 192 kHz for ultrasonic layers; the sample rate is shown in the header
- New "Gamma 40 Hz Pulses" preset and `GammaPulse` ping type: 1 ms tones at 10 kHz, 40 per second, as in Martorell et al. 2019
- explore_all: carriers moved to 300/307.5 Hz so its 220 Hz harmonic and 432 Hz pad no longer beat against them; adjusting the beat keeps the preset's carrier
- Deep Focus presets renamed to match their titles (`DEFAULT_deep_focus_adhd_alpha.json`, `DEFAULT_deep_focus_smr.json`); old copies are moved aside as `.bak`

## v0.3.0 (2026-11-25)

### Sound quality
- Tinnitus notch rebuilt as an order-8 Chebyshev II band-stop: at least 30 dB of cut across the whole labelled width (the old notch reached 20 dB over about a third of it)
- Square, saw and triangle waveforms are band-limited, so they no longer fold back into false audible tones
- Breath layer "Pink noise" now uses a real pink filter
- RF output keeps exact modulation frequencies (was 1.6% high at 48 kHz)
- Binaural beat range extended to 50 Hz, so the 40 Hz preset can be adjusted without dropping to 30 Hz
- The UI says when a ping is muted for being above the sound card's limit

### Real-time engine and recording
- The audio callback never allocates, blocks or writes to disk; WAV recording runs on its own thread
- Quitting during a recording now finishes the WAV file correctly

### Network streaming
- Choose the quality in the URL: `?fmt=s16` (default, dithered), `s24` or `f32` (bit-exact)
- Changing preset no longer disconnects listeners
- New listeners start about 200 ms behind live instead of 2 s

### Presets and citations
- Corrected the 40 Hz, tinnitus and RF preset texts against the published evidence
- Transmitting in 1400-1427 MHz (hydrogen line) is stated as prohibited worldwide (ITU RR 5.340)
- CITATIONS.md: fixed the Blanco-Duque 2024 authors, toned down unsupported claims, added 14 checked studies
- Built-in presets are refreshed on startup when a release changes them; the old copy is kept as a `.bak` file

### Documentation
- README key table fixed and new features documented; one-page overview in `docs/overview/`

## Dual-Path Architecture (2024-12-22)

### Major Features

#### Dual-Path Signal Generation
- **Separated acoustic and RF signal paths** for optimal output on each medium
- **Speaker path**: Uses psychoacoustic tricks (100 Hz AM by 7.83 Hz) for human listening
- **RF path**: Transmits TRUE 7.83 Hz envelope modulation (not AM approximation)
- Full 17 kHz ultrasonic bandwidth on RF (not limited by speakers)
- Independent signal processing for each output

#### Architecture Improvements
- `next_sample()` - Acoustic output optimized for speakers
- `next_rf_sample()` - RF output with true low-frequency envelopes
- No more compromise between speaker limitations and RF capabilities

### UI Improvements
- Renamed modes: `unknown-dogwhistle` → `uap-dogwhistle`
- Mode names use preset-style naming (human-focus10, uap-dogwhistle, etc.)
- Section headers simplified and clarified
- Column width increased from 28 to 40 characters for better readability
- Status bar compressed to 2 lines, shows more information
- Custom preset filenames displayed in status (e.g., "human-custom (8.5Hz) (custom_123.json)")

### Signal Layer Improvements
- **Signal layer now available in human-custom mode** (was locked before)
- Only Focus 10/12 preset modes lock signal layer
- Clear `[LOCKED]` indicator when signal layer is disabled
- Locked channels prevent volume adjustments, mute, and modulation changes

### Binaural Beats / Neural Coherence
- **Neural Coherence DISABLED in uap-dogwhistle mode** (binaural beats require headphones, not speakers!)
- Status bar shows 🎧 "USE HEADPHONES" reminder in human modes
- Neural Coherence volume adjustment blocked in uap-dogwhistle mode
- Clear UI indication: `[Disabled - not in human mode]`
- Documentation explains why binaural beats need stereo headphones

### Keybinding Changes
- **'m' key** - Mute/unmute selected channel (toggle volume 0% ↔ 50%)
- **'o' key** - Cycle oscillator/modulation type (was 'm')
- Clearer UI hints: `[m]ute [o]scillator` in status bar

### Preset System
- **Auto-creates presets folder** on first run
- Tracks loaded preset filename (except DEFAULT_ presets)
- Preset name cleared when manually changing modes
- Folder creation works even without presets/ directory

### Bug Fixes
- Fixed audio muting in human-custom mode (signal layer now works)
- Fixed mute logic to only apply in Focus 10/12, not custom modes
- Updated all header detection after box character changes

### Documentation
- Added DUAL_PATH_ARCHITECTURE.md with full technical explanation
- Updated README with dual-path overview
- Added comprehensive HackRF safety warnings
- Documented difference between "fake" 7.83 Hz (AM) and true RF envelope

---

## v0.2.0 - Initial Release

### Features
- Neural Coherence binaural beat generation (Focus 10, Focus 12, Custom)
- Signal layer frequencies (7.83 Hz Schumann, 528 Hz, 432 Hz, 17 kHz, chirps, breath)
- HackRF RF transmission support
- Preset save/load system
- TUI interface with live parameter adjustment
- Multiple oscillator types per channel

### Consciousness Modes
- Unknown (UAP) - Signal layer frequencies
- Human Focus 10 - 4.1 Hz theta binaural beat
- Human Focus 12 - 1.5 Hz delta binaural beat  
- Human Custom - User-defined binaural frequency

### RF Features
- Carrier: 1.42 GHz (Hydrogen Line) default
- Modulation: WBFM, NBFM, AM
- Adjustable gain: 0-47 dB
- Audio modulates RF carrier
