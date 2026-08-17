use crate::{BlockNode, ColumnAlignment, HighlightedToken, ImageSource, InlineNode, LinkTarget, SlugGenerator, TextStyle};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const HEADING_SIZES: [f32; 6] = [28.0, 22.0, 18.0, 16.0, 14.0, 12.0];
const DEFAULT_COLOR: [u8; 3] = [0, 0, 0];

/// Bundles the per-parse typography choices that need to reach deep into the recursive block/
/// inline lowering functions -- passed by reference alongside the existing `slugs`/
/// `next_diagram_id` state rather than as a `Stylesheet` directly, so this module only depends
/// on the exact pieces it uses.
struct Typography<'a> {
    heading: &'a sardown_style::HeadingStyle,
    body_size: f32,
    body_color: [u8; 3],
    body_font_family: String,
    table_cell_size: f32,
}

struct InlineBuilder {
    runs: Vec<InlineNode>,
    bold_depth: u32,
    italic_depth: u32,
    strikethrough_depth: u32,
    link_target: Option<LinkTarget>,
    base_size: f32,
    base_color: [u8; 3],
    base_font_family: String,
}

impl InlineBuilder {
    fn new(base_size: f32, base_color: [u8; 3], base_font_family: String) -> Self {
        Self { runs: Vec::new(), bold_depth: 0, italic_depth: 0, strikethrough_depth: 0, link_target: None, base_size, base_color, base_font_family }
    }

    fn push_text(&mut self, text: String) {
        self.push_text_with_font_family(text, self.base_font_family.clone());
    }

    /// Like `push_text`, but with an explicit font family instead of `base_font_family` --
    /// used for inline code spans, which always render monospace regardless of the surrounding
    /// text's own configured body font.
    fn push_text_with_font_family(&mut self, text: String, font_family: String) {
        if text.is_empty() {
            return;
        }
        self.runs.push(InlineNode {
            text,
            style: TextStyle {
                bold: self.bold_depth > 0,
                italic: self.italic_depth > 0,
                strikethrough: self.strikethrough_depth > 0,
                size: self.base_size,
                color: self.base_color,
                font_family,
            },
            link_target: self.link_target.clone(),
        });
    }
}

fn link_target_from_url(url: &str) -> LinkTarget {
    if let Some(anchor) = url.strip_prefix('#') {
        LinkTarget::InternalAnchor(anchor.to_string())
    } else {
        LinkTarget::ExternalUrl(url.to_string())
    }
}

/// Applies one inline event to `builder`. Shared between `lower_inline_events`'s main loop and
/// the paragraph-vs-standalone-image peek in `lower_block_events`, which needs to apply an
/// already-consumed "first event" before continuing the normal loop.
fn apply_inline_event(builder: &mut InlineBuilder, event: Event) {
    match event {
        Event::Text(text) => builder.push_text(text.into_string()),
        Event::Code(text) => builder.push_text_with_font_family(text.into_string(), "monospace".to_string()),
        Event::Start(Tag::Strong) => builder.bold_depth += 1,
        Event::End(TagEnd::Strong) => builder.bold_depth = builder.bold_depth.saturating_sub(1),
        Event::Start(Tag::Emphasis) => builder.italic_depth += 1,
        Event::End(TagEnd::Emphasis) => builder.italic_depth = builder.italic_depth.saturating_sub(1),
        Event::Start(Tag::Strikethrough) => builder.strikethrough_depth += 1,
        Event::End(TagEnd::Strikethrough) => builder.strikethrough_depth = builder.strikethrough_depth.saturating_sub(1),
        Event::Start(Tag::Link { dest_url, .. }) => {
            builder.link_target = Some(link_target_from_url(&dest_url));
        }
        Event::End(TagEnd::Link) => builder.link_target = None,
        Event::SoftBreak | Event::HardBreak => builder.push_text(" ".to_string()),
        // pulldown-cmark consumes the literal "[ ]"/"[x]" and reports it as this event instead --
        // without handling it, the checkbox disappears from the rendered text entirely rather
        // than falling back to the literal source text the way an un-enabled extension would.
        Event::TaskListMarker(checked) => builder.push_text(if checked { "\u{2611} ".to_string() } else { "\u{2610} ".to_string() }),
        _ => {}
    }
}

fn lower_inline_events<'a, I: Iterator<Item = Event<'a>>>(
    events: &mut std::iter::Peekable<I>,
    end_tag: TagEnd,
    base_size: f32,
    base_color: [u8; 3],
    base_font_family: String,
) -> Vec<InlineNode> {
    let mut builder = InlineBuilder::new(base_size, base_color, base_font_family);
    for event in events.by_ref() {
        if matches!(&event, Event::End(tag) if *tag == end_tag) {
            break;
        }
        apply_inline_event(&mut builder, event);
    }
    builder.runs
}

fn image_source_from_url(url: &str) -> ImageSource {
    if url.starts_with("http://") || url.starts_with("https://") {
        ImageSource::External(url.to_string())
    } else if url.starts_with("data:") {
        ImageSource::DataUri(url.to_string())
    } else {
        ImageSource::Embedded(std::path::PathBuf::from(url))
    }
}

fn lower_block_events<'a, I: Iterator<Item = Event<'a>>>(
    parser: &mut std::iter::Peekable<I>,
    end_tag: TagEnd,
    slugs: &mut SlugGenerator,
    next_diagram_id: &mut usize,
    diagram_positions: &mut std::vec::IntoIter<(usize, usize)>,
    typo: &Typography,
) -> Vec<BlockNode> {
    let mut blocks = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(tag) if tag == end_tag => break,
            // pulldown-cmark doesn't wrap a *tight* list item's (or any other tight context's)
            // inline content in Tag::Paragraph -- it emits bare inline events directly at the
            // block level (confirmed against pulldown-cmark's own event stream: "- one\n- two\n"
            // yields Start(Item), Text("one"), End(Item), with no Paragraph tag at all). Without
            // this arm, that content fell through to the catch-all below and was silently
            // dropped -- losing every item of the overwhelmingly common case of a list with no
            // blank lines between items. Collect it as an implicit paragraph, stopping (via
            // `peek`, so the triggering event is never consumed) at `end_tag` or at any event
            // that starts a real nested block, so those still get handled normally afterward.
            Event::Text(_)
            | Event::Code(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::TaskListMarker(_)
            | Event::Start(Tag::Strong)
            | Event::Start(Tag::Emphasis)
            | Event::Start(Tag::Strikethrough)
            | Event::Start(Tag::Link { .. }) => {
                let mut builder = InlineBuilder::new(typo.body_size, typo.body_color, typo.body_font_family.clone());
                apply_inline_event(&mut builder, event);
                loop {
                    match parser.peek() {
                        Some(Event::End(tag)) if *tag == end_tag => break,
                        Some(Event::Start(Tag::List(_) | Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::Table(_) | Tag::Heading { .. })) => break,
                        Some(Event::Rule) => break,
                        None => break,
                        _ => {}
                    }
                    apply_inline_event(&mut builder, parser.next().expect("just confirmed Some via peek"));
                }
                blocks.push(BlockNode::Paragraph { content: builder.runs });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level_u8 = heading_level_u8(level);
                let resolved = typo.heading.resolve(level_u8);
                let content = lower_inline_events(parser, TagEnd::Heading(level), resolved.size_pt, resolved.color.0, resolved.font_family.clone());
                let text: String = content.iter().map(|n| n.text.as_str()).collect::<Vec<_>>().join("");
                let id = slugs.generate(&text);
                blocks.push(BlockNode::Heading { level: level_u8, id, content });
            }
            Event::Start(Tag::Paragraph) => {
                // Images are inline events nested inside a Paragraph in pulldown-cmark's model,
                // but this AST treats a paragraph containing *only* an image as a block-level
                // Image node. Peek the first inline event to tell the two cases apart.
                match parser.next() {
                    Some(Event::Start(Tag::Image { dest_url, title, .. })) => {
                        let alt = collect_plain_text_until(parser, TagEnd::Image);
                        for event in parser.by_ref() {
                            if matches!(event, Event::End(TagEnd::Paragraph)) {
                                break;
                            }
                        }
                        let source = image_source_from_url(&dest_url);
                        let title = if title.is_empty() { None } else { Some(title.into_string()) };
                        blocks.push(BlockNode::Image { alt, title, source });
                    }
                    Some(first_event) => {
                        let mut builder = InlineBuilder::new(typo.body_size, typo.body_color, typo.body_font_family.clone());
                        if !matches!(&first_event, Event::End(tag) if *tag == TagEnd::Paragraph) {
                            apply_inline_event(&mut builder, first_event);
                            for event in parser.by_ref() {
                                if matches!(&event, Event::End(tag) if *tag == TagEnd::Paragraph) {
                                    break;
                                }
                                apply_inline_event(&mut builder, event);
                            }
                        }
                        blocks.push(BlockNode::Paragraph { content: builder.runs });
                    }
                    None => blocks.push(BlockNode::Paragraph { content: Vec::new() }),
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.into_string()),
                    _ => None,
                };
                let mut raw = String::new();
                for event in parser.by_ref() {
                    match event {
                        Event::Text(text) => raw.push_str(&text),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }
                if language.as_deref() == Some("mermaid") {
                    let id = format!("diagram-{next_diagram_id}");
                    *next_diagram_id += 1;
                    // Falls back to (1, 1) only if the pre-scan and the main lowering pass ever
                    // disagreed on how many mermaid fences exist -- shouldn't happen since both
                    // walk the same event stream, but a made-up position is safer than a panic.
                    let (line, column) = diagram_positions.next().unwrap_or((1, 1));
                    blocks.push(BlockNode::MermaidDiagram { id, source: raw, line, column, file: None });
                } else {
                    blocks.push(BlockNode::CodeBlock { language, tokens: vec![HighlightedToken { text: raw, color: DEFAULT_COLOR }] });
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let content = lower_block_events(parser, TagEnd::BlockQuote(None), slugs, next_diagram_id, diagram_positions, typo);
                blocks.push(BlockNode::Blockquote { content });
            }
            Event::Rule => blocks.push(BlockNode::ThematicBreak),
            Event::Start(Tag::List(start)) => {
                let ordered = start.is_some();
                let mut items = Vec::new();
                while let Some(event) = parser.next() {
                    match event {
                        Event::Start(Tag::Item) => {
                            items.push(lower_block_events(parser, TagEnd::Item, slugs, next_diagram_id, diagram_positions, typo));
                        }
                        Event::End(TagEnd::List(_)) => break,
                        _ => {}
                    }
                }
                blocks.push(BlockNode::List { ordered, start, items });
            }
            Event::Start(Tag::Table(alignment_spec)) => {
                let alignments = alignment_spec
                    .iter()
                    .map(|a| match a {
                        Alignment::Left => ColumnAlignment::Left,
                        Alignment::Center => ColumnAlignment::Center,
                        Alignment::Right => ColumnAlignment::Right,
                        Alignment::None => ColumnAlignment::None,
                    })
                    .collect();

                let mut headers = Vec::new();
                let mut rows = Vec::new();

                while let Some(event) = parser.next() {
                    match event {
                        Event::Start(Tag::TableHead) => {
                            headers = collect_table_cells(parser, TagEnd::TableHead, typo.table_cell_size, &typo.body_font_family);
                        }
                        Event::Start(Tag::TableRow) => {
                            let mut row = Vec::new();
                            while let Some(event) = parser.next() {
                                match event {
                                    Event::Start(Tag::TableCell) => {
                                        // One `Vec<InlineNode>` per cell, not flattened into the
                                        // row: a cell mixing plain text with a styled span (e.g.
                                        // inline code) produces more than one InlineNode, and
                                        // flattening lost the cell boundary — corrupting which
                                        // column every following run in the row landed in and
                                        // silently truncating whatever came after via `zip`.
                                        row.push(lower_inline_events(
                                            parser,
                                            TagEnd::TableCell,
                                            typo.table_cell_size,
                                            DEFAULT_COLOR,
                                            typo.body_font_family.clone(),
                                        ));
                                    }
                                    Event::End(TagEnd::TableRow) => break,
                                    _ => {}
                                }
                            }
                            rows.push(row);
                        }
                        Event::End(TagEnd::Table) => break,
                        _ => {}
                    }
                }

                blocks.push(BlockNode::Table { headers, rows, alignments });
            }
            _ => {}
        }
    }
    blocks
}

fn collect_table_cells<'a, I: Iterator<Item = Event<'a>>>(
    parser: &mut std::iter::Peekable<I>,
    end_tag: TagEnd,
    table_cell_size: f32,
    table_cell_font_family: &str,
) -> Vec<Vec<InlineNode>> {
    let mut cells = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                cells.push(lower_inline_events(parser, TagEnd::TableCell, table_cell_size, DEFAULT_COLOR, table_cell_font_family.to_string()));
            }
            Event::End(tag) if tag == end_tag => break,
            _ => {}
        }
    }
    cells
}

fn collect_plain_text_until<'a, I: Iterator<Item = Event<'a>>>(parser: &mut std::iter::Peekable<I>, end_tag: TagEnd) -> String {
    let mut text = String::new();
    for event in parser.by_ref() {
        match event {
            Event::Text(t) => text.push_str(&t),
            Event::End(tag) if tag == end_tag => break,
            _ => {}
        }
    }
    text
}

pub fn parse(markdown: &str) -> Vec<BlockNode> {
    let mut slugs = SlugGenerator::new();
    let mut next_diagram_id = 0usize;
    parse_with_slugs(markdown, &mut slugs, &mut next_diagram_id)
}

/// Like `parse`, but takes externally-owned slug and diagram-id state instead of creating fresh
/// state on every call. Used by sardown-book to parse multiple chapter files into one combined
/// document without heading ids or diagram ids from different chapters colliding.
pub fn parse_with_slugs(markdown: &str, slugs: &mut SlugGenerator, next_diagram_id: &mut usize) -> Vec<BlockNode> {
    parse_with_style(markdown, slugs, next_diagram_id, &sardown_style::Stylesheet::default())
}

/// Like `parse_with_slugs`, but takes a `Stylesheet` controlling heading and body typography
/// (size, color, and font family) and table-cell text size.
pub fn parse_with_style(markdown: &str, slugs: &mut SlugGenerator, next_diagram_id: &mut usize, style: &sardown_style::Stylesheet) -> Vec<BlockNode> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut diagram_positions = mermaid_diagram_positions(markdown).into_iter();
    let mut parser = Parser::new_ext(markdown, options).peekable();
    let typo = Typography {
        heading: &style.heading,
        body_size: style.typography.body_size_pt,
        body_color: style.typography.body_color.0,
        body_font_family: style.typography.font_family.clone(),
        table_cell_size: style.table.text_size_pt,
    };
    // TagEnd::Item is never opened at the top level, so it never matches; used only as a
    // sentinel that can't legitimately occur, meaning we consume until the iterator is exhausted.
    lower_block_events(&mut parser, TagEnd::Item, slugs, next_diagram_id, &mut diagram_positions, &typo)
}

/// 1-indexed (line, column) for a byte offset into `markdown`. Column counts characters (not
/// bytes) since the start of the line, matching how editors report cursor position.
fn line_col_at(markdown: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &markdown[..byte_offset.min(markdown.len())];
    let line = prefix.matches('\n').count() + 1;
    let column = match prefix.rfind('\n') {
        Some(i) => prefix[i + 1..].chars().count() + 1,
        None => prefix.chars().count() + 1,
    };
    (line, column)
}

/// The (line, column) of every mermaid fenced code block's opening fence, in the order they
/// appear. Found via a small, separate scan using pulldown-cmark's byte-offset-tracking
/// iterator, rather than threading offsets through the main lowering pass (which only ever
/// handles plain `Event`s, not `(Event, Range<usize>)` pairs). Mermaid diagrams are rare enough
/// per document that parsing the text twice is cheap, and this keeps the main, already-well-
/// tested lowering pipeline untouched.
fn mermaid_diagram_positions(markdown: &str) -> Vec<(usize, usize)> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    Parser::new_ext(markdown, options)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) if lang.as_ref() == "mermaid" => Some(line_col_at(markdown, range.start)),
            _ => None,
        })
        .collect()
}

/// Sets `file` on every `MermaidDiagram` in `blocks`, recursively (including inside blockquotes
/// and lists). `parse`/`parse_with_slugs` have no notion of "which file" a markdown string came
/// from -- callers that do (single-file rendering, per-chapter book combination) call this
/// afterward so a failed-diagram warning can point at a real file, not just a line number.
pub fn tag_diagram_origins(blocks: &mut [BlockNode], file: &std::path::Path) {
    for block in blocks {
        match block {
            BlockNode::MermaidDiagram { file: f, .. } => *f = Some(file.to_path_buf()),
            BlockNode::Blockquote { content } => tag_diagram_origins(content, file),
            BlockNode::List { items, .. } => {
                for item in items {
                    tag_diagram_origins(item, file);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    tag_diagram_origins(column, file);
                }
            }
            _ => {}
        }
    }
}

/// The `TextStyle` a heading of `level` gets from `parse()` -- same size table, non-bold,
/// non-italic, default color. Lets callers outside the parser (sardown-book, synthesizing a
/// chapter's title heading from its SUMMARY.md entry) build a `BlockNode::Heading` that matches
/// what parsing that same text as `# Title` would have produced, without duplicating
/// `HEADING_SIZES`.
pub fn heading_style_for_level(level: u8) -> TextStyle {
    let size = HEADING_SIZES[(level.clamp(1, 6) - 1) as usize];
    TextStyle { bold: false, italic: false, strikethrough: false, size, color: DEFAULT_COLOR, font_family: sardown_style::HeadingStyle::default().font_family }
}

fn heading_level_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
