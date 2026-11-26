mod columns;
mod parse;
mod slug;

pub use columns::group_columns;
pub use parse::{cited_footnote_ids, heading_style_for_level, parse, parse_with_slugs, parse_with_style, tag_diagram_origins};
pub use slug::{generate_heading_id, SlugGenerator};
use std::sync::Arc;

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
    /// A display equation (`$$ ... $$` alone in its paragraph). `source` is the LaTeX between the
    /// delimiters; `sardown-enrich` renders it to a vector graphic stored under `id` in the
    /// diagram table, and layout centers it like a diagram and anchors links at `id`. The id
    /// always contains a `:`, which a heading slug never does, so it cannot shadow a heading anchor.
    Math {
        id: String,
        source: String,
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
    /// A bibliography entry from front matter `references`. Transient: `parse_with_style` replaces
    /// the cited ones with a numbered "References" section and drops the rest, so layout never
    /// sees it.
    Reference {
        key: String,
        content: Vec<InlineNode>,
    },
    /// A centered line of text (title, author, date lines from front matter).
    Centered {
        content: Vec<InlineNode>,
    },
    /// A `Figure: ` / `Table: ` caption, already numbered. Laid out like a paragraph, but kept on
    /// one page with the image, table, equation or diagram directly after it, else directly before it.
    /// `id` is the anchor `@label` references link to, when the caption carries a `{#label}`.
    /// `wide` (a `{.wide}` attribute) makes a two-column layout span both columns with the block.
    Caption {
        id: Option<String>,
        content: Vec<InlineNode>,
        wide: bool,
    },
    /// A paragraph that links point at (a numbered bibliography entry); `id` is its anchor.
    Anchored {
        id: String,
        content: Vec<InlineNode>,
    },
    /// The text of one footnote, appended after all other blocks by the parser. `id` matches the
    /// `LinkTarget::Footnote` of the reference run that cites it; the layout places the text at
    /// the bottom of the page holding that reference, so this block itself draws nothing.
    FootnoteDefinition {
        id: String,
        content: Vec<InlineNode>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineNode {
    pub text: String,
    pub style: TextStyle,
    pub link_target: Option<LinkTarget>,
}

/// `font_family` is an `Arc<str>` rather than a `String`: every inline run carries its style,
/// and nearly all runs in a document share one of a handful of family names (body, heading,
/// monospace). A plain `String` meant a fresh heap allocation + memcpy of the same few names
/// for every text run at parse time, and again for every code token / list marker / measured
/// word downstream; `Arc<str>` turns each of those into a refcount bump.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub size: f32,
    pub color: [u8; 3],
    pub font_family: Arc<str>,
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
    /// A footnote reference mark; the id names the `BlockNode::FootnoteDefinition` it cites. Has
    /// no PDF link annotation -- the layout only uses it to place the footnote text.
    Footnote(String),
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
