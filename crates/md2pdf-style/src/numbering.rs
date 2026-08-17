#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberingFormat {
    #[default]
    Arabic,
    RomanLower,
    RomanUpper,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct PageNumbering {
    pub format: NumberingFormat,
    pub start_at: u32,
    /// Restarts numbering (with its own format/start_at) from a named heading's page onward --
    /// e.g. roman-numeral front matter followed by the body restarting at arabic 1. `at_heading`
    /// must match a real heading id or the reset is ignored with a warning at render time (there's
    /// no validation-time way to know which heading ids will exist).
    pub resets: Vec<PageNumberingReset>,
}

impl Default for PageNumbering {
    fn default() -> Self {
        PageNumbering { format: NumberingFormat::Arabic, start_at: 1, resets: Vec::new() }
    }
}

fn default_reset_start_at() -> u32 {
    1
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PageNumberingReset {
    pub at_heading: String,
    #[serde(default)]
    pub format: NumberingFormat,
    #[serde(default = "default_reset_start_at")]
    pub start_at: u32,
}
