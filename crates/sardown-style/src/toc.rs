#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TocStyle {
    pub enabled: bool,
    pub depth: u8,
    pub title: String,
}

impl Default for TocStyle {
    fn default() -> Self {
        TocStyle { enabled: false, depth: 2, title: "Table of Contents".to_string() }
    }
}
