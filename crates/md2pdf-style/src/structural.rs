use crate::Color;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct BlockquoteStyle {
    pub border_color: Color,
    pub border_width_pt: f32,
    pub indent_pt: f32,
}

impl Default for BlockquoteStyle {
    fn default() -> Self {
        BlockquoteStyle { border_color: Color([180, 180, 180]), border_width_pt: 2.0, indent_pt: 18.0 }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ThematicBreakStyle {
    pub color: Color,
    pub width_pt: f32,
}

impl Default for ThematicBreakStyle {
    fn default() -> Self {
        ThematicBreakStyle { color: Color([200, 200, 200]), width_pt: 1.0 }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ListStyle {
    pub indent_pt: f32,
}

impl Default for ListStyle {
    fn default() -> Self {
        ListStyle { indent_pt: 18.0 }
    }
}
