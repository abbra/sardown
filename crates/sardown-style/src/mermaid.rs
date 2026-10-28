use crate::Color;

/// Which engine turns a ```` ```mermaid ```` fence into a drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MermaidRenderer {
    /// Ondine's compiler: monochrome diagrams in a monospace font, colored from this section.
    /// Needs the `ondine` build feature; without it merman is used and a warning is printed.
    /// A diagram type Ondine does not support is drawn by merman instead.
    #[default]
    Ondine,
    /// merman: a port of Mermaid's own look (themes, `classDef`, `%%{init}%%`). Kept as the
    /// fallback for whatever Ondine does not draw, and selectable here.
    Merman,
}

/// `[mermaid]`: diagram rendering. The colors apply to the `ondine` renderer only; merman takes
/// its colors from the diagram's own theme.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct MermaidStyle {
    pub renderer: MermaidRenderer,
    /// Font for diagram text; the code block font when unset.
    pub font_family: Option<String>,
    /// Text color; the body text color when unset.
    pub text_color: Option<Color>,
    /// Edge and connector color; `text_color` when unset.
    pub line_color: Option<Color>,
    /// Node outline color; `text_color` when unset.
    pub border_color: Option<Color>,
    /// Node fill; `background` when unset.
    pub node_fill: Option<Color>,
    /// Fill of secondary shapes (clusters, headers, alternating rows); a tint of text over
    /// background when unset.
    pub surface_alt: Option<Color>,
    /// Accent color for highlighted parts; `text_color` when unset.
    pub accent: Option<Color>,
    /// Behind edge labels and node fills.
    pub background: Color,
    pub stroke_width_px: f32,
    /// Corner radius of nodes.
    pub radius_px: f32,
}

impl Default for MermaidStyle {
    fn default() -> Self {
        MermaidStyle {
            renderer: MermaidRenderer::Ondine,
            font_family: None,
            text_color: None,
            line_color: None,
            border_color: None,
            node_fill: None,
            surface_alt: None,
            accent: None,
            background: Color([255, 255, 255]),
            stroke_width_px: 1.5,
            radius_px: 0.0,
        }
    }
}
