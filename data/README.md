# Scientific Basis & Citations

The design of these presets is informed by the research outlined in the main `CITATIONS.md` file in the root of the repository. The "Focus" states (10, 12, 15, 21) are based on the foundational work of the Monroe Institute.

**[➡️ View the full list of citations in `../CITATIONS.md`](../CITATIONS.md)**

---

# Neural Coherence Research Presets

This directory contains frequency presets derived from scientific analysis of the Gateway Experience audio programs.

## Available Presets

### Focus 10 - "Mind Awake, Body Asleep"
**File:** `DEFAULT_focus_10_mind_awake.json`
- **Binaural Beat:** 4.11 Hz (theta)
- **Carriers:** 100.12 Hz (left) / 104.23 Hz (right)
- **Duration:** 5.5 minutes sustained state
- **Purpose:** Learn to maintain awareness while body deeply relaxes
- **Prerequisites:** None (starting point)

### Focus 12 - "Expanded Awareness"
**File:** `DEFAULT_focus_12_expanded.json`
- **Binaural Beat:** 1.50 Hz (deep delta)
- **Carriers:** 100.77 Hz (left) / 99.27 Hz (right)
- **Duration:** 18+ minutes sustained state
- **Purpose:** Expand awareness beyond physical senses, deep surrender
- **Prerequisites:** Focus 10 mastery

### Focus 15 - "No Time"
**File:** `DEFAULT_focus_15_no_time.json`
- **Binaural Beat:** 4.80 Hz (elevated theta)
- **Carriers:** 304.8 Hz (left) / 300.0 Hz (right) - **3x HIGHER than F10/F12!**
- **Duration:** 14 minutes total (fragmented sections)
- **Purpose:** Experience timeless consciousness, suspended awareness
- **Prerequisites:** Focus 10 and 12 experience
- **Note:** Higher carrier frequencies create unique quality - more "present" awareness

### Focus 21 - "Bridge to Other Realities"
**File:** `DEFAULT_focus_21_bridge.json`
- **Binaural Beat:** 4.00 Hz (theta/delta border - EXACT boundary)
- **Carriers:** 200.0 Hz (left) / 204.0 Hz (right) - **Balanced middle carriers**
- **Duration:** 15.7 minutes CONTINUOUS (longest sustained section!)
- **Purpose:** Out-of-body experiences, separation from physical body
- **Prerequisites:** Mastery of F10, F12, F15 + several months practice
- **Note:** This program was explicitly designed for OBE, referred to as the "Bridge to Other Realities," and is the culmination of the Gateway training. All previous levels build the skills for this stage.

### Expected Progression

**First Sessions:**
- Deep relaxation
- Body numbness
- Hypnagogic imagery
- Time distortion

**After Practice:**
- Vibrations/energy sensations
- Floating feelings
- Auditory changes
- Visual phenomena

**With Mastery:**
- Actual separation sensations
- Movement in non-physical
- Exploration capability
- Controlled navigation

---

## Technical Notes

### Why Different Carriers Matter

**100 Hz (F10/F12):**
- Below conscious audio perception threshold
- "Subliminal" quality
- Deep, submerged feeling
- Good for initial training

**200 Hz (F21):**
- Right at perceptual threshold
- Octave of 100 Hz (harmonic relationship)
- Balanced awareness
- **Optimal for OBE** - aware but not too alert

**300 Hz (F15):**
- Clearly perceptible
- More conscious awareness of tone
- Creates "suspended" state
- Good for time distortion
- May be too alert for full OBE

### The 4 Hz Sweet Spot

All theta-based Focus levels cluster around 4 Hz:
- F10: 4.11 Hz
- F15: 4.80 Hz
- **F21: 4.00 Hz** ← EXACTLY at theta/delta border

**Why 4.0 Hz is special:**
- Boundary between theta (4-8 Hz) and delta (0.5-4 Hz)
- Deep enough for body separation
- Alert enough for conscious navigation
- Research shows 4-5 Hz optimal for OBE induction

---

## Research Documentation

See `/research/` directory for detailed analysis:

- `FOCUS_15_ANALYSIS.md` - Complete Focus 15 frequency breakdown
- `FOCUS_21_ANALYSIS.md` - Complete Focus 21 frequency breakdown
- `MONROE_INSTITUTE_RESEARCH.md` - Monroe Institute background
- `ALPHA_THETA_RESEARCH_SUMMARY.md` - General binaural beat research
- `ANALYZER_VALIDATION.md` - Validation of analysis methodology

---

## Preset File Format

Each preset is JSON format:

```json
{
  "name": "Focus Level Name",
  "binaural_beat_hz": X.XX,
  "carriers": {
    "left_hz": XXX.X,
    "right_hz": XXX.X
  },
  "focus_level": XX,
  "technical_details": { ... },
  "usage_notes": [ ... ]
}
```

---

## Disclaimer

**Important Notes:**

- ⚠️ These are research/educational presets
- ⚠️ Monroe Institute trademark - these are decoded for study
- ⚠️ NOT medical devices or treatments
- ⚠️ Individual results vary significantly
- ⚠️ Requires proper training and practice
- ⚠️ Start with F10, progress gradually
- ⚠️ Do not use while driving or operating machinery
- ⚠️ Consult healthcare provider if you have neurological conditions

**Prerequisites for F21:**
- Several MONTHS of practice with F10, F12, F15
- Comfortable with altered states
- No fear of separation sensations
- Understand this is serious consciousness work

---

**Presets Decoded:** December 2025
**Analysis Method:** FFT Spectral Analysis
**Accuracy:** ±0.05 Hz
**Validation:** High confidence (see research documentation)

---

## Related Documentation

### Understanding the Data
- [DATA_INTEGRATION_GUIDE.md](../docs/DATA_INTEGRATION_GUIDE.md) - **How to interpret JSON analysis files**
- [COMPLETE_FREQUENCY_MAP.md](../docs/COMPLETE_FREQUENCY_MAP.md) - Master frequency reference
- [CITATIONS.md](../CITATIONS.md) - Full bibliography (21 peer-reviewed sources)

### Detailed Analysis
- [FOCUS_10_ANALYSIS.md](../research/FOCUS_10_ANALYSIS.md) - Complete Focus 10 breakdown
- [FOCUS_15_ANALYSIS.md](../research/FOCUS_15_ANALYSIS.md) - Complete Focus 15 breakdown
- [FOCUS_21_ANALYSIS.md](../research/FOCUS_21_ANALYSIS.md) - Complete Focus 21 breakdown (OBE)
- [MONROE_INSTITUTE_RESEARCH.md](../research/MONROE_INSTITUTE_RESEARCH.md) - Historical context
- [ALPHA_THETA_RESEARCH_SUMMARY.md](../research/ALPHA_THETA_RESEARCH_SUMMARY.md) - Binaural beat science

### Technical Documentation
- [ANALYZER_VALIDATION.md](../docs/ANALYZER_VALIDATION.md) - Methodology validation
- [COHERENCE_DECODE.md](../docs/COHERENCE_DECODE.md) - FFT analysis tool docs
