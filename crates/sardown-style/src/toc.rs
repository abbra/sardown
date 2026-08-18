#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TocStyle {
    pub enabled: bool,
    pub depth: u8,
    pub title: String,
    /// Whether the table of contents also gets an in-document page of its own. `enabled` alone
    /// always populates the PDF's bookmark/outline panel from the same heading list; setting this
    /// to `false` skips inserting the rendered TOC page (and the page-number shift it otherwise
    /// causes) while keeping that outline, for readers who navigate via the PDF viewer's sidebar
    /// rather than a printed contents page.
    pub page: bool,
}

impl Default for TocStyle {
    fn default() -> Self {
        TocStyle { enabled: false, depth: 2, title: "Table of Contents".to_string(), page: true }
    }
}
