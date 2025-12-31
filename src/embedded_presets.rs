/// Embedded default presets - compiled into the binary
/// These serve as fallbacks when user config directory doesn't have the preset

pub struct EmbeddedPreset {
    pub filename: &'static str,
    pub content: &'static str,
}

pub const EMBEDDED_PRESETS: &[EmbeddedPreset] = &[
    EmbeddedPreset {
        filename: "DEFAULT_actual_dog_whistle.json",
        content: include_str!("../presets/DEFAULT_actual_dog_whistle.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_deep_focus_active.json",
        content: include_str!("../presets/DEFAULT_deep_focus_active.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_deep_focus_adhd.json",
        content: include_str!("../presets/DEFAULT_deep_focus_adhd.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_deep_focus_calm.json",
        content: include_str!("../presets/DEFAULT_deep_focus_calm.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_deep_focus_peak.json",
        content: include_str!("../presets/DEFAULT_deep_focus_peak.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_explore_all.json",
        content: include_str!("../presets/DEFAULT_explore_all.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_focus_10_mind_awake.json",
        content: include_str!("../presets/DEFAULT_focus_10_mind_awake.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_focus_12_expanded.json",
        content: include_str!("../presets/DEFAULT_focus_12_expanded.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_focus_15_no_time.json",
        content: include_str!("../presets/DEFAULT_focus_15_no_time.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_focus_21_bridge.json",
        content: include_str!("../presets/DEFAULT_focus_21_bridge.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_brycehelm.json",
        content: include_str!("../presets/DEFAULT_uap_brycehelm.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_enigmatic_ideas.json",
        content: include_str!("../presets/DEFAULT_uap_enigmatic_ideas.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_frequencies.json",
        content: include_str!("../presets/DEFAULT_uap_frequencies.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_rf_ultrasonic.json",
        content: include_str!("../presets/DEFAULT_uap_rf_ultrasonic.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_sweep_18khz.json",
        content: include_str!("../presets/DEFAULT_uap_sweep_18khz.json"),
    },
    EmbeddedPreset {
        filename: "DEFAULT_uap_sweep_24khz.json",
        content: include_str!("../presets/DEFAULT_uap_sweep_24khz.json"),
    },
];
