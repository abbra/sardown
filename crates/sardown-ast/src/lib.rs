mod columns;
mod parse;
mod slug;
pub use columns::group_columns;
pub use parse::{heading_style_for_level, parse, parse_with_slugs, parse_with_style, tag_diagram_origins};
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
    MermaidDiagram {
        id: String,
        source: String,
        line: usize,
        column: usize,
        file: Option<std::path::PathBuf>,
    },
    Image {
        alt: String,
        title: Option<String>,
        source: ImageSource,
    },
    Table {
        headers: Vec<Vec<InlineNode>>,
        rows: Vec<Vec<Vec<InlineNode>>>,
        alignments: Vec<ColumnAlignment>,
    },
    /// `start` is the literal number the list's first item was written with (CommonMark honors
    /// this -- "5. Fifth" starts numbering at 5, not 1), and is `None` for an unordered list.
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<Vec<BlockNode>>,
    },
    /// One `Vec<BlockNode>` per column, in source order. Never produced by the core parser --
    /// built by the separate `group_columns` post-parse transform from `::columns`/`::column`/
    /// `::end` sentinel paragraphs. See `group_columns`'s own doc comment for the exact syntax.
    Columns(Vec<Vec<BlockNode>>),
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
    pub strikethrough: bool,
    pub size: f32,
    pub color: [u8; 3],
    pub font_family: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    InternalAnchor(String),
    ExternalUrl(String),
    /// Transient: produced by sardown-book while parsing a chapter, for a relative link that
    /// names another chapter in the same book. Rewritten to `InternalAnchor` (or dropped, if
    /// unresolvable) once the whole book's heading-slug map is known -- never reaches
    /// sardown-layout or sardown-pdf in practice; see `links::build_annotation`'s arm for this
    /// variant for the defensive fallback if that invariant is ever violated.
    CrossFileAnchor {
        file: std::path::PathBuf,
        fragment: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageSource {
    Embedded(std::path::PathBuf),
    External(String),
    /// A `data:` URI with the image bytes inlined directly in the document (e.g.
    /// `data:image/png;base64,...`), rather than referencing a file or a remote URL. Stored as
    /// the raw URI string; decoding happens in `sardown-layout`, matching how `Embedded`'s path is
    /// only resolved/decoded later rather than eagerly here.
    DataUri(String),
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
