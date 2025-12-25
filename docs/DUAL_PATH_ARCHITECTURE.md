# Dual-Path Signal Architecture

SoulWhistle implements **separate signal paths** for acoustic (speaker) and electromagnetic (RF) output to overcome the physical limitations of speakers while providing true frequency transmission via RF.

## The Problem

### Speaker Limitations:
- **Cannot produce true 7.83 Hz** - Wavelength is 43.8 meters, requires massive subwoofer
- **17 kHz ultrasonic limited** - Most speakers roll off above 15-16 kHz
- **Psychoacoustic tricks needed** - Use AM modulation to create perception of low frequencies

### RF Capabilities:
- **Can transmit true 7.83 Hz envelope** - As modulation on GHz carrier
- **Full ultrasonic bandwidth** - No speaker cone limitations
- **Electromagnetic propagation** - Not limited by acoustic physics

## Solution: Dual Signal Paths

### PATH 1: Acoustic Output (Speakers)

**File:** `src/audio.rs` → `next_sample()`

**Purpose:** Human consciousness alteration via binaural beats and audible frequencies

**Signal Processing:**
```
7.83 Hz Schumann:  100 Hz AM by 7.83 Hz (psychoacoustic trick)
                   Creates perception of 7.83 Hz rhythm
                   Actual frequencies: ~92, 100, 108 Hz

528 Hz Harmonic:   Clean sine wave (audible)
432 Hz Pad:        Clean sine wave (audible)
2.5 kHz Chirps:    Clear, audible pulses
17 kHz Ping:       Marginal (speaker-dependent, may be filtered)
Breath Layer:      Shaped noise (audible)
```

**When Used:**
- All modes when listening through speakers
- Provides audible "monitoring" of what's being transmitted on RF
- Human meditation/consciousness exploration (Focus 10/12/custom)

---

### PATH 2: Electromagnetic Output (HackRF)

**File:** `src/audio.rs` → `next_rf_sample()`

**Purpose:** True electromagnetic transmission for "UAP communication" research

**Signal Processing:**
```
7.83 Hz Schumann:  TRUE 7.83 Hz envelope (not AM trick!)
                   Direct amplitude modulation of RF carrier
                   Creates: 1.42 GHz ± 7.83 Hz sidebands

528 Hz Harmonic:   Full fidelity modulation
432 Hz Pad:        Full fidelity modulation
2.5 kHz Chirps:    Full dynamic range
17 kHz Ping:       FULL strength (not limited by speakers)
Breath Layer:      Full bandwidth noise shaping
```

**When Used:**
- Only when HackRF transmission is enabled
- Completely independent from speaker output
- True electromagnetic transmission following physics

---

## Technical Implementation

### Speaker Path (Psychoacoustic):
```rust
// 100 Hz carrier amplitude modulated by 7.83 Hz
SignalType::SchumannAM => 
    self.phase_100hz.sin() * (0.5 + 0.5 * self.phase_7_83hz.sin())

// Result: 92 Hz, 100 Hz, 108 Hz acoustic waves
// Brain perceives 7.83 Hz pulsing rhythm
```

### RF Path (True 7.83 Hz):
```rust
// Direct 7.83 Hz envelope for RF modulation
let schumann_envelope = 0.5 + 0.5 * self.phase_7_83hz.sin();
let carrier_signal = schumann_envelope * params.carrier_vol;

// Result: 1.42 GHz carrier ± 7.83 Hz EM sidebands
// Actual 7.83 Hz information in electromagnetic spectrum
```

---

## Why Binaural Beats Need Headphones

**Binaural beats are NOT sound waves** - they're a neurological phenomenon:

1. **Left ear**: Hears 100 Hz
2. **Right ear**: Hears 104 Hz (4 Hz difference)
3. **Brain**: Perceives 4 Hz "beat" (theta wave entrainment)

**Why speakers don't work:**
- Both frequencies mix in the air BEFORE reaching your ears
- Your brain receives the already-mixed signal
- No frequency difference between ears = no binaural effect
- You just hear a warbling 100 Hz tone

**Why headphones work:**
- Each ear receives ONLY its designated frequency
- No air mixing before reaching ears
- Brain processes the frequency difference
- Creates perceived beat at 4 Hz (Focus 10), 1.5 Hz (Focus 12), etc.

**In SoulWhistle:**
- **uap-dogwhistle mode**: No binaural beats, speakers OK
- **human modes**: Binaural beats active, **headphones REQUIRED**
- Status bar shows 🎧 reminder in human modes

---

## Mode Behavior

### uap-dogwhistle Mode:

**Speakers:**
- Plays all signal layer components
- 7.83 Hz is psychoacoustic (100 Hz AM)
- 17 kHz may be weak/filtered
- **Neural Coherence DISABLED** (binaural beats require headphones, not speakers!)

**RF (when enabled):**
- Transmits TRUE 7.83 Hz envelope
- Full 17 kHz ultrasonic component
- All frequencies at full fidelity
- No binaural beats (signal layer only)

---

### human-focus10 / human-focus12 Modes:

**Speakers:**
- Plays binaural beats (100 Hz carriers with theta/delta beats)
- **REQUIRES HEADPHONES** (left ear ≠ right ear for binaural effect)
- Signal layer MUTED and LOCKED
- 🎧 Status bar shows "USE HEADPHONES" reminder

**RF (when enabled):**
- Transmits ONLY binaural beat information
- Signal layer muted (Focus presets are pure binaural)

---

### human-custom Mode:

**Speakers:**
- Plays custom binaural frequency
- **REQUIRES HEADPHONES** (binaural effect needs stereo)
- Signal layer available (user adjustable)
- 7.83 Hz is psychoacoustic (100 Hz AM)
- 🎧 Status bar shows "USE HEADPHONES" reminder

**RF (when enabled):**
- Transmits custom binaural frequency
- Signal layer available with TRUE 7.83 Hz
- Full experimental control

---

## Spectrum Analysis

### What a Spectrum Analyzer Shows:

**Speaker Output (uap-dogwhistle):**
```
~92 Hz   - Lower sideband of 100 Hz AM
100 Hz   - Carrier (AM by 7.83 Hz)
~108 Hz  - Upper sideband
432 Hz   - Ambient pad
528 Hz   - Harmonic
2.5 kHz  - Chirps (periodic)
~17 kHz  - Ultrasonic ping (weak, may not show)
```

**RF Output (uap-dogwhistle at 1.42 GHz):**
```
1.419999923 GHz  - Lower sideband (1.42 GHz - 7.83 Hz)
1.420000000 GHz  - Carrier
1.420000077 GHz  - Upper sideband (1.42 GHz + 7.83 Hz)
+ Additional sidebands at ±432 Hz, ±528 Hz, ±2.5 kHz, ±17 kHz
```

**The difference:** RF has TRUE 7.83 Hz information, speakers have ~100 Hz approximation.

---

## Volume Considerations

**Claim:** "Volume doesn't matter in this context"

**Engineering Reality:**
- ❌ **Unproven speculation** - No scientific backing
- ✅ **Physics applies** - Inverse square law, noise floor, SNR requirements
- ✅ **Higher power = greater range** - For both acoustic and EM transmission
- ✅ **Detection requires minimum signal** - Any receiver needs threshold power

**For Research:**
- Start with safe/legal power levels
- Increase systematically if testing detection range
- Document power levels in research notes

---

## Advantages of This Architecture

1. **Acoustic Path:**
   - Works on any speaker system
   - Provides consciousness alteration for human listeners
   - Audible monitoring of transmitted signal
   - Safe, no regulatory concerns

2. **RF Path:**
   - True 7.83 Hz Schumann resonance modulation
   - Full ultrasonic bandwidth (17 kHz+)
   - Electromagnetic propagation (detectable by radio receivers)
   - Follows actual physics of EM transmission

3. **Flexibility:**
   - Use speakers-only for meditation (no HackRF needed)
   - Use RF for electromagnetic experiments
   - Use both simultaneously (monitor + transmit)
   - Independent control of each path

---

## Safety & Legal

**Speakers:**
- No regulations
- Safe for personal use
- Recommended for consciousness exploration

**RF Transmission:**
- ⚠️ **REQUIRES FCC LICENSE** (or equivalent in your country)
- ⚠️ **Never use during flight** (interference with aircraft systems)
- ⚠️ **Can interfere with GPS, radio astronomy, emergency services**
- ⚠️ **Know your local regulations** - 1.42 GHz (Hydrogen Line) is protected
- ⚠️ **Penalties are severe** - Fines and criminal prosecution possible

---

## For Researchers

When documenting experiments:

**Acoustic Experiments:**
- Note speaker specifications (frequency response)
- Document volume levels (dB SPL)
- Record listening duration
- Note subjective effects

**RF Experiments:**
- Document carrier frequency (default 1.42 GHz)
- Record transmission power (HackRF gain setting)
- Note modulation type (WBFM, NBFM, AM)
- Document legal authorization (license number)
- Record environmental conditions
- Use proper RF safety practices

---

## Summary

**Speakers = Psychoacoustic approximation for human consciousness**
- Good for: Meditation, binaural beats, personal exploration
- Limited by: Speaker physics, acoustic propagation

**RF = True electromagnetic transmission for detection experiments**
- Good for: EM "communication" experiments, full bandwidth
- Limited by: Legal requirements, safety concerns, receiver needed

**Together = Complete research platform**
- Monitor acoustically what you're transmitting electromagnetically
- Explore both human consciousness AND EM transmission hypotheses
- Scientific documentation of both signal paths
