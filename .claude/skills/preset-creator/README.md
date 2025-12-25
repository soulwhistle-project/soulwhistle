# Preset Creator Skill

This skill provides expert guidance for creating scientifically-grounded binaural beat presets for Soulwhistle.

## What is this?

The Preset Creator is a Claude Code skill that contains comprehensive knowledge about:
- Peer-reviewed binaural beat research
- Monroe Institute decoded frequency protocols
- Optimal carrier frequency ranges
- Multi-phase session design
- Safety guidelines and contraindications
- Individual variation factors

## How to Use

### For Claude Code CLI Users

If you're using Claude Code CLI, this skill is automatically available:

```bash
# Just invoke it naturally in your conversation
"Create a preset for deep meditation using theta waves"
"Design a focus preset for ADHD based on Reedijk et al. 2013"
"Make a preset similar to Monroe Focus 10"
```

The skill will:
1. Ask clarifying questions about your use case
2. Recommend research-based frequencies
3. Generate a complete JSON preset file
4. Explain the scientific basis for choices
5. Include appropriate safety warnings

### For Manual Preset Creation

If you're not using Claude Code CLI, refer to `SKILL.md` for:
- Complete frequency guidelines
- Research citations
- JSON structure templates
- Safety considerations

## Key Features

✅ **Research-Grounded**: All recommendations based on peer-reviewed studies or established protocols

✅ **Safety-Focused**: Includes contraindications, usage limits, and individual variation warnings

✅ **Optimal Engineering**: Ensures proper carrier frequencies (300-600 Hz) and session structure

✅ **Multi-Phase Design**: Implements Startup → Induction → Stabilization → Return phases

## Example Usage

**User**: "Create a focus preset for coding sessions"

**Skill Response**:
- Asks about preferred intensity (calm focus vs high alertness)
- Recommends 18 Hz beta (Lane et al. 1998) or 10 Hz alpha (for calm focus)
- Generates JSON with optimal 400 Hz carrier
- Includes 40-min session structure
- Warns about individual dopamine variation

## Contributing

If you improve this skill or find research that should be added, please submit a PR! See the main `CONTRIBUTING.md` for guidelines.

## Research Sources

All recommendations in this skill are based on:
- `../../CITATIONS.md` - Peer-reviewed research compilation
- `../../research/` - Monroe Institute decoded frequencies
- `../../docs/` - Additional technical documentation

## License

This skill is part of the Soulwhistle project and follows the same license.
