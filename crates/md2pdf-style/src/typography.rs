use crate::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TypographyStyle {
    pub font_family: String,
    pub font_dirs: Vec<std::path::PathBuf>,
    pub use_system_fonts: bool,
    pub body_size_pt: f32,
    pub body_color: Color,
    pub alignment: TextAlignment,
}

impl Default for TypographyStyle {
    fn default() -> Self {
        TypographyStyle {
            font_family: "sans-serif".to_string(),
            font_dirs: Vec::new(),
            use_system_fonts: true,
            body_size_pt: 12.0,
            body_color: Color([0, 0, 0]),
            alignment: TextAlignment::Left,
        }
    }
}
