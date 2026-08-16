use crate::Color;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct HeaderZones {
    pub left: String,
    pub center: String,
    pub right: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderFooterMode {
    #[default]
    Uniform,
    TwoSided,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct HeaderFooterStyle {
    pub enabled: bool,
    pub font_family: String,
    pub font_size_pt: f32,
    pub color: Color,
    pub mode: HeaderFooterMode,
    pub suppress_on_chapter_start: bool,
    pub uniform: HeaderZones,
    pub odd: HeaderZones,
    pub even: HeaderZones,
}

impl Default for HeaderFooterStyle {
    fn default() -> Self {
        HeaderFooterStyle {
            enabled: false,
            font_family: "sans-serif".to_string(),
            font_size_pt: 9.0,
            color: Color([102, 102, 102]),
            mode: HeaderFooterMode::Uniform,
            suppress_on_chapter_start: true,
            uniform: HeaderZones::default(),
            odd: HeaderZones::default(),
            even: HeaderZones::default(),
        }
    }
}
