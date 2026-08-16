mod color;
mod heading;
mod page;
mod structural;
mod table;
mod typography;

pub use color::Color;
pub use heading::{HeadingLevelStyle, HeadingStyle, ResolvedHeadingStyle};
pub use page::{PageFormat, PageStyle};
pub use structural::{BlockquoteStyle, ListStyle, ThematicBreakStyle};
pub use table::TableStyle;
pub use typography::TypographyStyle;
