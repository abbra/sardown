/// Footnote text placed at the bottom of the page that cites it.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct FootnoteStyle {
    pub size_pt: f32,
    /// Space left between two footnotes on the same page.
    pub spacing_pt: f32,
}

impl Default for FootnoteStyle {
    fn default() -> Self {
        FootnoteStyle { size_pt: 9.0, spacing_pt: 2.0 }
    }
}
