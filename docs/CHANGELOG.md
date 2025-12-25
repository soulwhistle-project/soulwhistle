# Changelog

## v0.3.0 - Dual-Path Architecture (2024-12-22)

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
