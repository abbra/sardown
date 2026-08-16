use crate::Color;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TypographyStyle {
    pub font_family: String,
    pub font_dirs: Vec<std::path::PathBuf>,
    pub use_system_fonts: bool,
    pub body_size_pt: f32,
    pub body_color: Color,
}

impl Default for TypographyStyle {
    fn default() -> Self {
        TypographyStyle {
            font_family: "sans-serif".to_string(),
            font_dirs: Vec::new(),
            use_system_fonts: true,
            body_size_pt: 12.0,
            body_color: Color([0, 0, 0]),
        }
    }
}
