use crate::{Color, TextAlignment};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
}

/// One named slide layout's overrides on top of the document's own `[typography]`/`[heading]`.
/// Every field absent (the default) means "inherit the base document's own value" -- see
/// `md2pdf-slides`' `build_slide_stylesheet` for how these are applied.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct SlideLayoutStyle {
    pub alignment: Option<TextAlignment>,
    pub vertical_align: VerticalAlign,
    pub body_size_pt: Option<f32>,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub suppress_header: bool,
    pub suppress_footer: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct SlidesStyle {
    /// The layout every slide uses unless it has its own `@layout: <name>` directive. `None`
    /// means every slide falls back to one fully built-in layout (see `md2pdf-slides`'
    /// `resolve_layout`) -- only meaningful when no layout name is referenced anywhere at all.
    pub default_layout: Option<String>,
    /// Auto-shrink-to-fit floor: the smallest scale (relative to the document's own configured
    /// font sizes) a slide's text is ever shrunk to before giving up and rendering the overflow.
    pub min_scale: f32,
    pub layouts: BTreeMap<String, SlideLayoutStyle>,
}

impl Default for SlidesStyle {
    fn default() -> Self {
        SlidesStyle { default_layout: None, min_scale: 0.5, layouts: BTreeMap::new() }
    }
}
