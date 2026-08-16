mod code_block;
mod color;
mod heading;
mod page;
mod structural;
mod table;
mod typography;

pub use code_block::{CodeBlockDefaultStyle, CodeBlockStyle, CodeLanguageStyle, LabelStyle, ResolvedCodeBlockStyle};
pub use color::Color;
pub use heading::{HeadingLevelStyle, HeadingStyle, ResolvedHeadingStyle};
pub use page::{PageFormat, PageStyle};
pub use structural::{BlockquoteStyle, ListStyle, ThematicBreakStyle};
pub use table::TableStyle;
pub use typography::TypographyStyle;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct Stylesheet {
    pub page: PageStyle,
    pub typography: TypographyStyle,
    pub heading: HeadingStyle,
    pub blockquote: BlockquoteStyle,
    pub thematic_break: ThematicBreakStyle,
    pub list: ListStyle,
    pub table: TableStyle,
    pub code_block: CodeBlockStyle,
}

impl Stylesheet {
    fn validate(&self) -> anyhow::Result<()> {
        match (self.page.width_mm, self.page.height_mm) {
            (Some(_), None) => anyhow::bail!("[page] sets width_mm but not height_mm -- set both or neither"),
            (None, Some(_)) => anyhow::bail!("[page] sets height_mm but not width_mm -- set both or neither"),
            _ => Ok(()),
        }
    }
}
