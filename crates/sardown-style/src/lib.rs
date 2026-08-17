use anyhow::Context;

mod code_block;
mod color;
mod document;
mod header_footer;
mod heading;
mod numbering;
mod page;
mod slides;
mod structural;
mod table;
mod toc;
mod typography;

pub use code_block::{CodeBlockDefaultStyle, CodeBlockStyle, CodeLanguageStyle, LabelStyle, ResolvedCodeBlockStyle};
pub use color::Color;
pub use document::DocumentStyle;
pub use header_footer::{HeaderFooterMode, HeaderFooterStyle, HeaderZones};
pub use heading::{HeadingLevelStyle, HeadingStyle, ResolvedHeadingStyle};
pub use numbering::{NumberingFormat, PageNumbering, PageNumberingReset};
pub use page::{PageFormat, PageStyle};
pub use slides::{BackgroundImageStyle, ImageCorner, SlideLayoutStyle, SlidesStyle, VerticalAlign};
pub use structural::{BlockquoteStyle, ColumnsStyle, ListStyle, ThematicBreakStyle};
pub use table::TableStyle;
pub use toc::TocStyle;
pub use typography::{TextAlignment, TypographyStyle};

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct Stylesheet {
    pub document: DocumentStyle,
    pub page: PageStyle,
    pub typography: TypographyStyle,
    pub heading: HeadingStyle,
    pub blockquote: BlockquoteStyle,
    pub thematic_break: ThematicBreakStyle,
    pub list: ListStyle,
    pub columns: ColumnsStyle,
    pub table: TableStyle,
    pub code_block: CodeBlockStyle,
    pub header: HeaderFooterStyle,
    pub footer: HeaderFooterStyle,
    pub toc: TocStyle,
    pub slides: SlidesStyle,
}

impl Stylesheet {
    pub fn resolve(explicit_path: Option<&std::path::Path>, book_root: Option<&std::path::Path>) -> anyhow::Result<Stylesheet> {
        if let Some(path) = explicit_path {
            return Stylesheet::load(path);
        }
        if let Some(root) = book_root {
            let candidate = root.join("style.toml");
            if candidate.is_file() {
                return Stylesheet::load(&candidate);
            }
        }
        Ok(Stylesheet::default())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Stylesheet> {
        let text = std::fs::read_to_string(path).with_context(|| format!("failed to read stylesheet {}", path.display()))?;
        let sheet: Stylesheet = toml::from_str(&text).with_context(|| format!("failed to parse stylesheet {}", path.display()))?;
        sheet.validate().with_context(|| format!("invalid stylesheet {}", path.display()))?;
        Ok(sheet)
    }

    fn validate(&self) -> anyhow::Result<()> {
        match (self.page.width_mm, self.page.height_mm) {
            (Some(_), None) => anyhow::bail!("[page] sets width_mm but not height_mm -- set both or neither"),
            (None, Some(_)) => anyhow::bail!("[page] sets height_mm but not width_mm -- set both or neither"),
            _ => {}
        }
        match (self.page.inner_margin_mm, self.page.outer_margin_mm) {
            (Some(_), None) => anyhow::bail!("[page] sets inner_margin_mm but not outer_margin_mm -- set both or neither"),
            (None, Some(_)) => anyhow::bail!("[page] sets outer_margin_mm but not inner_margin_mm -- set both or neither"),
            _ => {}
        }
        self.header.validate("header")?;
        self.footer.validate("footer")?;
        if !(1..=6).contains(&self.toc.depth) {
            anyhow::bail!("[toc] depth must be between 1 and 6, got {}", self.toc.depth);
        }
        if let Some(name) = &self.slides.default_layout {
            if !self.slides.layouts.contains_key(name) {
                anyhow::bail!("[slides] default_layout = {name:?} has no matching [slides.layouts.{name}] table");
            }
        }
        if self.slides.min_scale <= 0.0 || self.slides.min_scale > 1.0 {
            anyhow::bail!("[slides] min_scale must be greater than 0.0 and at most 1.0, got {}", self.slides.min_scale);
        }
        Ok(())
    }
}
