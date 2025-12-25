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
- ⚠️ No sample rate override or audio device selection
- ⚠️ No real-time Nyquist limit warnings

**Pull requests for piezo transducer integration are highly encouraged!**

## OpenBCI Integration (Planned)

A primary goal is to integrate with OpenBCI hardware for closed-loop brainwave entrainment research. This would enable:
- Real-time EEG monitoring during sessions
- Automatic frequency adjustment based on brain state
- Data correlation between stimulus and response
- Reproducible experimental protocols

**Pull requests for OpenBCI integration are highly encouraged!**
