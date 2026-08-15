mod parse;
mod slug;
pub use parse::parse;
pub use slug::{generate_heading_id, SlugGenerator};

#[derive(Debug, Clone, PartialEq)]
pub enum BlockNode {
    Heading {
        level: u8,
        id: String,
        content: Vec<InlineNode>,
    },
    Paragraph {
        content: Vec<InlineNode>,
    },
    CodeBlock {
        language: Option<String>,
        tokens: Vec<HighlightedToken>,
    },
    Blockquote {
        content: Vec<BlockNode>,
    },
    ThematicBreak,
    PageBreak,
    MermaidDiagram { id: String, source: String },
    Image { alt: String, title: Option<String>, source: ImageSource },
    Table { headers: Vec<Vec<InlineNode>>, rows: Vec<Vec<Vec<InlineNode>>>, alignments: Vec<ColumnAlignment> },
    List { ordered: bool, items: Vec<Vec<BlockNode>> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineNode {
    pub text: String,
    pub style: TextStyle,
    pub link_target: Option<LinkTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub size: f32,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    InternalAnchor(String),
    ExternalUrl(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageSource {
    Embedded(std::path::PathBuf),
    External(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnAlignment {
    Left,
    Center,
    Right,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HighlightedToken {
    pub text: String,
    pub color: [u8; 3],
}
