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
}

impl Default for PageNumbering {
    fn default() -> Self {
        PageNumbering { format: NumberingFormat::Arabic, start_at: 1 }
    }
}
