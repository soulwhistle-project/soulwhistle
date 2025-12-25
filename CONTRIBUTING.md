# Contributing to Soulwhistle

We welcome contributions from the community! Whether you're fixing bugs, improving documentation, or creating new binaural beat presets, your contributions help make Soulwhistle better.

## Table of Contents

- [Reporting Bugs](#reporting-bugs)
- [Suggesting Enhancements](#suggesting-enhancements)
- [Contributing Presets](#contributing-presets)
- [Pull Request Process](#pull-request-process)
- [Code Contributions](#code-contributions)

---

## Reporting Bugs

Please open an issue on GitHub with:
- Clear description of the bug
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error messages
- Your environment (OS, Rust version, hardware)

---

## Suggesting Enhancements

Open an issue with the "enhancement" label to discuss your ideas. Include:
- Use case and motivation
- Proposed solution or approach
- Any relevant research or references

---

## Contributing Presets

We welcome both research-based and experimental binaural beat presets!

- **Research presets**: Must be grounded in peer-reviewed research or established frequency protocols
- **Experimental presets**: Exploratory frequency combinations based on observations, anecdotal reports, or hypotheses - clearly marked with `"experimental": true`

### Prerequisites

1. **Familiarize yourself with the research**:
   - Read `CITATIONS.md` for peer-reviewed binaural beat research
   - Review `research/` directory for Monroe Institute decoded frequencies
   - Study existing presets in `presets/` directory

2. **Use the Preset Creator Skill** (recommended):
   - The `.claude/skills/preset-creator/SKILL.md` contains expert knowledge for creating scientifically-grounded presets
   - If using Claude Code CLI, the skill is automatically available
   - The skill ensures proper frequency ranges, carrier optimization, and safety guidelines

### Preset Requirements

All preset submissions MUST include:

#### 1. **Research Citation OR Experimental Disclaimer**

**For research-based presets:**
- At least one peer-reviewed study OR established frequency protocol (e.g., Monroe Institute)
- Include full citation in `preset_description` field
- Add detailed citation to `CITATIONS.md` if not already present

**For experimental presets:**
- Set `"experimental": true` in the JSON
- Clearly explain the hypothesis or observation in `preset_description`
- Include appropriate disclaimers about lack of peer review

#### 2. **Clear Use Case**
- Specific intended effect (e.g., "10 Hz alpha for relaxed focus")
- Target audience (e.g., "ADHD support", "meditation", "creative work")
- Expected session duration

#### 3. **Safety Warnings** (if applicable)
- Contraindications (e.g., "NOT recommended for epilepsy")
- Usage limits (e.g., "15 min max for 40 Hz gamma")
- Individual variation disclaimers

#### 4. **Proper Naming Convention**
- Format: `DEFAULT_category_descriptor.json`
- Examples:
  - `DEFAULT_deep_focus_active.json` (18 Hz beta for active focus)
  - `DEFAULT_meditation_theta.json` (6 Hz theta for meditation)
  - `DEFAULT_sleep_delta.json` (2 Hz delta for sleep)

#### 5. **Complete JSON Structure**

**Research preset example:**
```json
{
  "preset_title": "Your Preset Title (Frequency & State)",
  "preset_description": "Detailed description with research citation, use case, warnings, and usage guidelines. Include: target frequency, brainwave state, research basis (e.g., Author et al. YEAR), optimal duration, contraindications, and individual variation notes.",
  "lock_signal_layer": true,
  "master_vol": 0.6,
  "coherence": {
    "enabled": true,
    "being_type": "HumanCustom",
    "left_carrier": 400.0,
    "right_carrier": 410.0,
    "custom_binaural_hz": 10.0,
    "volume": 0.6,
    "startup_duration_min": 2.0,
    "induction_duration_min": 15.0,
    "stabilization_duration_min": 20.0,
    "return_duration_min": 5.0
  }
}
```

**Experimental preset example:**
```json
{
  "preset_title": "Experimental Frequency Exploration",
  "preset_description": "EXPERIMENTAL: This preset explores specific frequency combinations based on [observation/hypothesis]. Not peer-reviewed. Use with caution and discontinue if adverse effects occur.",
  "experimental": true,
  "lock_signal_layer": true,
  "master_vol": 0.6,
  "coherence": {
    "enabled": true,
    "being_type": "HumanCustom",
    "left_carrier": 400.0,
    "right_carrier": 410.0,
    "custom_binaural_hz": 10.0,
    "volume": 0.6,
    "startup_duration_min": 2.0,
    "induction_duration_min": 15.0,
    "stabilization_duration_min": 20.0,
    "return_duration_min": 5.0
  }
}
```

### Frequency Guidelines

**MUST follow research-based ranges**:

- **Delta (0.5-4 Hz)**: Deep sleep, meditation
  - Carrier: 350-400 Hz
  - Duration: 20-30 min minimum

- **Theta (4-8 Hz)**: Meditation, creativity
  - Carrier: 350-400 Hz
  - Duration: 20-30 min

- **Alpha (8-12 Hz)**: Relaxed focus
  - Carrier: 400-450 Hz
  - Duration: 30-45 min
  - Note individual dopamine variation (Reedijk et al. 2013)

- **Beta (12-30 Hz)**: Active focus, attention
  - Carrier: 400-450 Hz
  - Duration: 30-40 min

- **Gamma (30-100 Hz)**: Peak attention (use sparingly!)
  - Carrier: 440-500 Hz
  - Duration: 10-15 min MAX
  - ⚠️ High risk of overstimulation

**Carrier Frequency Rules**:
- ✅ 300-600 Hz range (optimal)
- ❌ < 300 Hz (less robust)
- ❌ > 1000 Hz (binaural effect breaks down)

### Testing Your Preset

Before submitting:

1. **Test the preset yourself** for at least 3 sessions
2. **Verify JSON validity**: Use `cargo check` or JSON validator
3. **Check description clarity**: Can someone unfamiliar understand the use case and warnings?
4. **Confirm research basis**: Is the citation accurate and verifiable?

### Submitting Your Preset

1. **Fork the repository**
2. **Create a new branch**:
   ```bash
   git checkout -b preset/your-preset-name
   ```

3. **Add your preset file** to `presets/` directory:
   ```bash
   git add presets/DEFAULT_your_preset.json
   ```

4. **Update CITATIONS.md** (if adding new research):
   ```bash
   git add CITATIONS.md
   ```

5. **Commit with clear message**:
   ```bash
   git commit -m "Add preset: Brief description

   - Frequency: X Hz (state)
   - Use case: Specific use case
   - Research: Author et al. (YEAR)
   - Duration: XX min"
   ```

6. **Push to your fork**:
   ```bash
   git push origin preset/your-preset-name
   ```

7. **Open a Pull Request** with:
   - **Title**: `Preset: [Your Preset Name]`
   - **Description**:
     - Research basis and citation
     - Intended use case
     - Testing notes (how many sessions, effects observed)
     - Any special considerations or warnings

### Preset Review Criteria

Your preset will be reviewed for:

✅ **Scientific validity**: Grounded in research (for research presets) OR clearly marked as experimental with rationale
✅ **Safety**: Appropriate warnings and contraindications
✅ **Clarity**: Description is clear and informative
✅ **Technical correctness**: Valid JSON, optimal carrier frequencies
✅ **Usefulness**: Fills a genuine need or use case

**Note**:
- Research presets must include peer-reviewed citations
- Experimental presets must set `"experimental": true` and include clear disclaimers
- Presets with potentially harmful parameters will not be accepted regardless of type

---

## Pull Request Process

1. Ensure your code/preset adheres to project standards
2. Update documentation as needed
3. Test your changes thoroughly
4. Write clear commit messages
5. Respond to review feedback promptly

---

## Code Contributions

For code contributions:
- Follow existing code style (Rust)
- Add tests for new features
- Update documentation
- Ensure `cargo check` and `cargo build` pass
- Keep commits focused and atomic

---

## Questions?

Open an issue with the "question" label or join discussions in the repository.

Thank you for contributing to Soulwhistle and advancing consciousness research! 🧠✨
