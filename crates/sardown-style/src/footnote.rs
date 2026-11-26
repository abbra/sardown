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

impl FootnoteStyle {
    pub fn validate(&self, section_name: &str) -> anyhow::Result<()> {
        if self.size_pt <= 0.0 {
            anyhow::bail!("[{section_name}] size_pt must be greater than 0.0, got {}", self.size_pt);
        }
        if self.spacing_pt < 0.0 {
            anyhow::bail!("[{section_name}] spacing_pt must be at least 0.0, got {}", self.spacing_pt);
        }
        Ok(())
    }
}
