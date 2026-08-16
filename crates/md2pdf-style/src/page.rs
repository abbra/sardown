/// A named page size preset. `Letter` (the current, and only, size md2pdf has ever produced) is
/// the default so an absent `[page]` section changes nothing about existing output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageFormat {
    #[default]
    Letter,
    A4,
    A3,
    A5,
    Legal,
}

impl PageFormat {
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            PageFormat::Letter => (215.9, 279.4),
            PageFormat::A4 => (210.0, 297.0),
            PageFormat::A3 => (297.0, 420.0),
            PageFormat::A5 => (148.0, 210.0),
            PageFormat::Legal => (215.9, 355.6),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct PageStyle {
    pub format: PageFormat,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
    pub margin_mm: f32,
    pub numbering: crate::PageNumbering,
}

impl Default for PageStyle {
    fn default() -> Self {
        PageStyle {
            format: PageFormat::Letter,
            width_mm: None,
            height_mm: None,
            margin_mm: 25.4,
            numbering: crate::PageNumbering::default(),
        }
    }
}

impl PageStyle {
    /// Explicit `width_mm`/`height_mm` (both must be set together -- enforced by
    /// `Stylesheet::validate`, not here) override `format`'s preset entirely.
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match (self.width_mm, self.height_mm) {
            (Some(w), Some(h)) => (w, h),
            _ => self.format.dimensions_mm(),
        }
    }
}
