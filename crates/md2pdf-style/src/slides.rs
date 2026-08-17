use crate::{Color, TextAlignment};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageCorner {
    TopLeft,
    TopRight,
    #[default]
    BottomLeft,
    BottomRight,
}

fn default_background_image_width_pt() -> f32 {
    60.0
}

fn default_background_image_margin_pt() -> f32 {
    14.0
}

/// One decorative image drawn in a corner of every slide using a given layout, on top of
/// `background_color` and behind all slide content. `path` is resolved the same way embedded
/// Markdown images are -- relative to the input file, constrained to stay within its directory --
/// and may point at either a raster image or an `.svg` file; `render_slide_deck` checks both
/// tables and draws whichever one actually decoded. `path` has no default: a background image
/// entry with no path makes no sense, so TOML omitting it is a deserialization error rather than a
/// silently broken empty path.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct BackgroundImageStyle {
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub corner: ImageCorner,
    // Chosen as reasonable defaults for a small corner logo/watermark, not zero (which
    // `#[derive(Default)]` would otherwise give an untouched f32 field, silently drawing the
    // image at zero size).
    #[serde(default = "default_background_image_width_pt")]
    pub width_pt: f32,
    #[serde(default = "default_background_image_margin_pt")]
    pub margin_pt: f32,
}

/// One named slide layout's overrides on top of the document's own `[typography]`/`[heading]`.
/// Every field absent (the default) means "inherit the base document's own value" -- see
/// `md2pdf-slides`' `build_slide_stylesheet`/`rescale_slide_content` for how these are applied.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct SlideLayoutStyle {
    pub alignment: Option<TextAlignment>,
    pub vertical_align: VerticalAlign,
    pub body_size_pt: Option<f32>,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    /// Applied to non-bold paragraph/list-item text only -- headings and **bold** runs always use
    /// `text_color`. Falls back to `text_color` when unset, so a layout that only sets
    /// `text_color` behaves exactly as before this field existed.
    pub secondary_text_color: Option<Color>,
    pub suppress_header: bool,
    pub suppress_footer: bool,
    /// Zero or more decorative images (raster or SVG), expressed in TOML as
    /// `[[slides.layouts.<name>.background_images]]` array-of-tables entries.
    pub background_images: Vec<BackgroundImageStyle>,
}

impl Default for SlideLayoutStyle {
    fn default() -> Self {
        SlideLayoutStyle {
            alignment: None,
            vertical_align: VerticalAlign::default(),
            body_size_pt: None,
            background_color: None,
            text_color: None,
            secondary_text_color: None,
            suppress_header: false,
            suppress_footer: false,
            background_images: Vec::new(),
        }
    }
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
