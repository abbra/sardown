use crate::{BlockNode, ColumnAlignment, HighlightedToken, ImageSource, InlineNode, LinkTarget, SlugGenerator, TextStyle};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, MetadataBlockKind, Options, Parser, Tag, TagEnd};

use std::collections::HashMap;
use std::sync::Arc;

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
    link_color: Option<[u8; 3]>,
    body_font_family: Arc<str>,
    /// Font for inline code spans: the stylesheet's code-block font, so `code` in prose matches
    /// fenced blocks instead of whatever the system's generic "monospace" resolves to.
    code_font_family: Arc<str>,
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
    link_color: Option<[u8; 3]>,
    base_font_family: Arc<str>,
    code_font_family: Arc<str>,
}

impl InlineBuilder {
    fn new(base_size: f32, base_color: [u8; 3], link_color: Option<[u8; 3]>, base_font_family: Arc<str>, code_font_family: Arc<str>) -> Self {
        Self {
            runs: Vec::new(),
            bold_depth: 0,
            italic_depth: 0,
            strikethrough_depth: 0,
            link_target: None,
            base_size,
            base_color,
            link_color,
            base_font_family,
            code_font_family,
        }
    }

    fn push_text(&mut self, text: String) {
        self.push_text_with_font_family(text, self.base_font_family.clone());
    }

    /// Like `push_text`, but with an explicit font family instead of `base_font_family` --
    /// used for inline code spans, which always render monospace regardless of the surrounding
    /// text's own configured body font.
    fn push_text_with_font_family(&mut self, text: String, font_family: Arc<str>) {
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
                color: if self.link_target.is_some() { self.link_color.unwrap_or(self.base_color) } else { self.base_color },
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
        Event::Code(text) => {
            let family = builder.code_font_family.clone();
            builder.push_text_with_font_family(text.into_string(), family)
        }
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
        // Math inside running text stays text (italic LaTeX source): only whole-paragraph
        // `$$...$$` equations are typeset, because a line of text cannot carry an inline graphic.
        Event::InlineMath(text) | Event::DisplayMath(text) => {
            builder.italic_depth += 1;
            builder.push_text(text.into_string());
            builder.italic_depth -= 1;
        }
        // Placeholder: `resolve_footnotes` rewrites the label to a number once the whole document
        // has been parsed (numbers follow reference order, not definition order).
        Event::FootnoteReference(label) => {
            let label = label.into_string();
            builder.push_text(label.clone());
            if let Some(run) = builder.runs.last_mut() {
                run.link_target = Some(LinkTarget::Footnote(label));
            }
        }
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
    link_color: Option<[u8; 3]>,
    base_font_family: Arc<str>,
    code_font_family: Arc<str>,
) -> Vec<InlineNode> {
    let mut builder = InlineBuilder::new(base_size, base_color, link_color, base_font_family, code_font_family);
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
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::TaskListMarker(_)
            | Event::Start(Tag::Strong)
            | Event::Start(Tag::Emphasis)
            | Event::Start(Tag::Strikethrough)
            | Event::Start(Tag::Link { .. }) => {
                let mut builder =
                    InlineBuilder::new(typo.body_size, typo.body_color, typo.link_color, typo.body_font_family.clone(), typo.code_font_family.clone());
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
            Event::Start(Tag::MetadataBlock(_)) => {
                let yaml = collect_plain_text_until(parser, TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle));
                match serde_saphyr::from_str::<FrontMatter>(&yaml) {
                    Ok(front) => lower_front_matter(front, &mut blocks, slugs, next_diagram_id, typo),
                    Err(e) => eprintln!("warning: ignoring front matter: {e}"),
                }
            }
            Event::Start(Tag::Heading { level, id: explicit_id, .. }) => {
                let level_u8 = heading_level_u8(level);
                let resolved = typo.heading.resolve(level_u8);
                let mut content = lower_inline_events(
                    parser,
                    TagEnd::Heading(level),
                    resolved.size_pt,
                    resolved.color.0,
                    typo.link_color,
                    resolved.font_family.as_str().into(),
                    typo.code_font_family.clone(),
                );
                if resolved.bold {
                    content.iter_mut().for_each(|n| n.style.bold = true);
                }
                let text: String = content.iter().map(|n| n.text.as_str()).collect::<Vec<_>>().join("");
                // An explicit `{#id}` is the cross-reference label; the author keeps it unique.
                let id = explicit_id.map(|id| id.into_string()).unwrap_or_else(|| slugs.generate(&text));
                blocks.push(BlockNode::Heading { level: level_u8, id, content });
            }
            Event::Start(Tag::Paragraph) => {
                // Images are inline events nested inside a Paragraph in pulldown-cmark's model,
                // but this AST treats a paragraph containing *only* an image as a block-level
                // Image node. Peek the first inline event to tell the two cases apart.
                match parser.next() {
                    Some(Event::DisplayMath(source)) if matches!(parser.peek(), Some(Event::End(TagEnd::Paragraph))) => {
                        parser.next();
                        let id = format!("math:{next_diagram_id}");
                        *next_diagram_id += 1;
                        blocks.push(BlockNode::Math { id, source: source.trim().to_string() });
                    }
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
                        let mut builder =
                            InlineBuilder::new(typo.body_size, typo.body_color, typo.link_color, typo.body_font_family.clone(), typo.code_font_family.clone());
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
            Event::Start(Tag::FootnoteDefinition(label)) => {
                let body = lower_block_events(parser, TagEnd::FootnoteDefinition, slugs, next_diagram_id, diagram_positions, typo);
                let mut content: Vec<InlineNode> = Vec::new();
                for block in body {
                    if let BlockNode::Paragraph { content: paragraph } = block {
                        if let (Some(_), Some(first)) = (content.last(), paragraph.first()) {
                            let mut space = first.clone();
                            space.text = " ".to_string();
                            space.link_target = None;
                            content.push(space);
                        }
                        content.extend(paragraph);
                    }
                }
                blocks.push(BlockNode::FootnoteDefinition { id: label.into_string(), content });
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
                            headers = collect_table_cells(
                                parser,
                                TagEnd::TableHead,
                                typo.table_cell_size,
                                typo.link_color,
                                &typo.body_font_family,
                                &typo.code_font_family,
                            );
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
                                            typo.link_color,
                                            typo.body_font_family.clone(),
                                            typo.code_font_family.clone(),
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
    link_color: Option<[u8; 3]>,
    table_cell_font_family: &str,
    code_font_family: &Arc<str>,
) -> Vec<Vec<InlineNode>> {
    let mut cells = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                cells.push(lower_inline_events(
                    parser,
                    TagEnd::TableCell,
                    table_cell_size,
                    DEFAULT_COLOR,
                    link_color,
                    table_cell_font_family.into(),
                    code_font_family.clone(),
                ));
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
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    // A ```mermaid fence implies the substring "mermaid", so this cheap scan gates the second
    // full pulldown-cmark pass below: documents without a single diagram (the overwhelming
    // majority) skip the extra tokenization entirely -- the same "gate expensive setup when the
    // feature is absent" pattern the highlighter's ast_contains_code_block gate uses.
    let mut diagram_positions = if markdown.contains("mermaid") { mermaid_diagram_positions(markdown).into_iter() } else { Vec::new().into_iter() };
    let typo = Typography {
        heading: &style.heading,
        body_size: style.typography.body_size_pt,
        body_color: style.typography.body_color.0,
        link_color: style.typography.link_color.map(|c| c.0),
        body_font_family: style.typography.font_family.as_str().into(),
        code_font_family: style.code_block.default.font_family.as_str().into(),
        table_cell_size: style.table.text_size_pt,
    };
    // TagEnd::Item is never opened at the top level, so it never matches; used only as a
    let mut parser = Parser::new_ext(markdown, options).peekable();
    // sentinel that can't legitimately occur, meaning we consume until the iterator is exhausted.
    let mut blocks = lower_block_events(&mut parser, TagEnd::Item, slugs, next_diagram_id, &mut diagram_positions, &typo);
    resolve_footnotes(&mut blocks, next_diagram_id);
    number_document(&mut blocks, style);
    // After numbering, so the generated References heading stays unnumbered.
    resolve_citations(&mut blocks, slugs, &typo, style.code_block.default.font_family.as_str(), next_diagram_id);
    blocks
}

/// YAML front matter. `author` and `authors` both take a string or a list; each entry is one
/// centered line, so an affiliation can follow the name after a comma.
#[derive(serde::Deserialize, Default)]
struct FrontMatter {
    title: Option<String>,
    #[serde(default, alias = "author")]
    authors: OneOrMany,
    date: Option<String>,
    r#abstract: Option<String>,
    /// Bibliography entries keyed by citation key; each value is one Markdown paragraph.
    #[serde(default)]
    references: std::collections::BTreeMap<String, String>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl Default for OneOrMany {
    fn default() -> Self {
        OneOrMany::Many(Vec::new())
    }
}

/// Title in the level-1 heading style, then the author and date lines, then an "Abstract" label
/// over the abstract (lowered as Markdown) in a blockquote, so the stylesheet's `[blockquote]`
/// indent gives the narrower abstract measure.
fn lower_front_matter(front: FrontMatter, blocks: &mut Vec<BlockNode>, slugs: &mut SlugGenerator, next_diagram_id: &mut usize, typo: &Typography) {
    let line = |text: String, size: f32, color: [u8; 3], font: Arc<str>, bold: bool| BlockNode::Centered {
        content: vec![InlineNode { text, style: TextStyle { bold, italic: false, strikethrough: false, size, color, font_family: font }, link_target: None }],
    };
    if let Some(title) = front.title {
        let h = typo.heading.resolve(1);
        blocks.push(line(title, h.size_pt, h.color.0, h.font_family.as_str().into(), h.bold));
    }
    let authors = match front.authors {
        OneOrMany::One(a) => vec![a],
        OneOrMany::Many(a) => a,
    };
    let body = |text: String, bold: bool| line(text, typo.body_size, typo.body_color, typo.body_font_family.clone(), bold);
    blocks.extend(authors.into_iter().chain(front.date).map(|text| body(text, false)));
    if let Some(text) = front.r#abstract {
        blocks.push(body("Abstract".to_string(), true));
        let mut options = Options::empty();
        options.insert(Options::ENABLE_MATH);
        let mut parser = Parser::new_ext(&text, options).peekable();
        let content = lower_block_events(&mut parser, TagEnd::Item, slugs, next_diagram_id, &mut Vec::new().into_iter(), typo);
        blocks.push(BlockNode::Blockquote { content });
    }
    for (key, text) in front.references {
        let mut parser = Parser::new_ext(&text, Options::empty()).peekable();
        let entry = lower_block_events(&mut parser, TagEnd::Item, slugs, next_diagram_id, &mut Vec::new().into_iter(), typo);
        if let Some(BlockNode::Paragraph { content }) = entry.into_iter().next() {
            blocks.push(BlockNode::Reference { key, content });
        }
    }
}

fn for_each_inlines(blocks: &mut [BlockNode], f: &mut impl FnMut(&mut Vec<InlineNode>)) {
    for block in blocks {
        match block {
            BlockNode::Heading { content, .. }
            | BlockNode::Paragraph { content }
            | BlockNode::Centered { content }
            | BlockNode::Caption { content, .. }
            | BlockNode::Anchored { content, .. }
            | BlockNode::FootnoteDefinition { content, .. } => f(content),
            BlockNode::Blockquote { content } => for_each_inlines(content, f),
            BlockNode::List { items, .. } => items.iter_mut().for_each(|item| for_each_inlines(item, f)),
            BlockNode::Columns(columns) => columns.iter_mut().for_each(|column| for_each_inlines(column, f)),
            BlockNode::Table { headers, rows, .. } => {
                headers.iter_mut().for_each(&mut *f);
                rows.iter_mut().flatten().for_each(&mut *f);
            }
            _ => {}
        }
    }
}

/// Auto-numbers sections (per `heading.number_from_level`), `Figure:`/`Table:` captions, and
/// equations carrying `\label{..}`, then resolves `@label` mentions in the text to
/// "Section 2.1", "Figure 3", "Table 1" or "Eq. (4)". Labels come from heading ids (`{#sec:x}`),
/// a trailing `{#fig:x}` on a caption, and `\label{eq:x}` in a `$$` block. A mention naming no
/// label stays as written, so e-mail addresses and handles survive.
fn number_document(blocks: &mut [BlockNode], style: &sardown_style::Stylesheet) {
    let from = style.heading.number_from_level;
    let mut section = [0u32; 7];
    let (mut figures, mut tables, mut equations) = (0u32, 0u32, 0u32);
    // label -> (text it expands to, heading anchor to link to)
    let mut labels: HashMap<String, (String, Option<String>)> = HashMap::new();
    for block in blocks.iter_mut() {
        match block {
            BlockNode::Heading { level, id, content } if from > 0 && *level >= from => {
                let depth = (*level - from) as usize;
                section[depth] += 1;
                section[depth + 1..].fill(0);
                // A heading that skips a level (`##` straight after `#`, or a document starting
                // at `##`) would otherwise number against a parent counter that never
                // incremented ("1.0.1"); each unincremented parent counts as 1.
                for parent in &mut section[..depth] {
                    if *parent == 0 {
                        *parent = 1;
                    }
                }
                let number = section[..=depth].iter().map(u32::to_string).collect::<Vec<_>>().join(".");
                labels.insert(id.clone(), (format!("Section {number}"), Some(id.clone())));
                if let Some(first) = content.first().cloned() {
                    content.insert(0, InlineNode { text: format!("{number} "), link_target: None, ..first });
                }
            }
            BlockNode::Paragraph { content } => {
                let Some(first) = content.first_mut() else { continue };
                let (kind, count) = if first.text.starts_with("Figure: ") {
                    ("Figure", &mut figures)
                } else if first.text.starts_with("Table: ") {
                    ("Table", &mut tables)
                } else {
                    continue;
                };
                *count += 1;
                first.text.drain(..kind.len() + 2);
                let mut label = InlineNode { text: format!("{kind} {count}. "), ..first.clone() };
                label.style.bold = true;
                let mut id = None;
                let mut wide = false;
                if let Some(last) = content.last_mut() {
                    let trimmed = last.text.trim_end();
                    // A trailing `{#label .wide}`: any order, at least one of the two.
                    let attributes = trimmed.strip_suffix('}').and_then(|s| s.rsplit_once('{'));
                    if let Some((prefix, attrs)) = attributes
                        .filter(|(_, attrs)| !attrs.trim().is_empty() && attrs.split_whitespace().all(|w| w == ".wide" || (w.starts_with('#') && w.len() > 1)))
                    {
                        wide = attrs.split_whitespace().any(|w| w == ".wide");
                        if let Some(name) = attrs.split_whitespace().find_map(|w| w.strip_prefix('#')) {
                            let anchor = format!("label:{name}");
                            labels.insert(name.to_string(), (format!("{kind} {count}"), Some(anchor.clone())));
                            id = Some(anchor);
                        }
                        last.text.truncate(prefix.trim_end().len());
                    }
                }
                content.insert(0, label);
                let content = std::mem::take(content);
                *block = BlockNode::Caption { id, content, wide };
            }
            BlockNode::Math { id, source } => {
                let Some(start) = source.find("\\label{") else { continue };
                let Some(len) = source[start..].find('}') else { continue };
                equations += 1;
                let name = &source[start + 7..start + len];
                *id = format!("label:{name}");
                labels.insert(name.to_string(), (format!("Eq. ({equations})"), Some(id.clone())));
                source.replace_range(start..start + len + 1, "");
                source.push_str(&format!(" \\qquad\\tag{{{equations}}}"));
            }
            _ => {}
        }
    }
    if labels.is_empty() {
        return;
    }
    let code_font = style.code_block.default.font_family.as_str();
    for_each_inlines(blocks, &mut |inlines| {
        splice_matches(inlines, code_font, '@', &mut |text, from| {
            let mut search = from;
            while let Some(at) = text[search..].find('@').map(|i| i + search) {
                search = at + 1;
                let word_end = text[at + 1..].find(|c: char| !(c.is_alphanumeric() || ":_-.".contains(c))).map_or(text.len(), |i| at + 1 + i);
                let name = text[at + 1..word_end].trim_end_matches(['.', ':', '-', '_']);
                let preceded_by_word = text[..at].chars().next_back().is_some_and(char::is_alphanumeric);
                if let Some((display, anchor)) = labels.get(name).filter(|_| !preceded_by_word) {
                    return Some((at, at + 1 + name.len(), vec![(display.clone(), anchor.clone().map(LinkTarget::InternalAnchor))]));
                }
            }
            None
        });
    });
}

/// Replaces matches inside plain (not linked, not code) text runs. `find(text, from)` returns the
/// next `(start, end, replacement pieces, each with its link)` at or after `from`; runs without `marker` are skipped
/// without calling it. Each match splits its run into before / replacement / after, all keeping
/// the run's style.
fn splice_matches(
    inlines: &mut Vec<InlineNode>,
    code_font: &str,
    marker: char,
    find: &mut impl FnMut(&str, usize) -> Option<(usize, usize, Vec<(String, Option<LinkTarget>)>)>,
) {
    let mut out = Vec::with_capacity(inlines.len());
    for node in inlines.drain(..) {
        if node.link_target.is_some() || node.style.font_family.as_ref() == code_font || !node.text.contains(marker) {
            out.push(node);
            continue;
        }
        let text = node.text.as_str();
        let mut rest = 0;
        while let Some((start, end, pieces)) = find(text, rest) {
            for (piece, target) in std::iter::once((text[rest..start].to_string(), None)).chain(pieces) {
                if !piece.is_empty() {
                    out.push(InlineNode { text: piece, link_target: target, ..node.clone() });
                }
            }
            rest = end;
        }
        if rest == 0 {
            out.push(node);
        } else if rest < text.len() {
            out.push(InlineNode { text: text[rest..].to_string(), ..node.clone() });
        }
    }
    *inlines = out;
}

/// Resolves `[@key]` / `[@a; @b]` citations against front matter `references`: numbers entries in
/// order of first citation, rewrites each citation to "[1]" / "[1, 2]", and appends a
/// "References" heading with one "[n] entry" paragraph per cited entry. Entries nothing cites are
/// dropped; a citation of an unknown key is left as written, with a warning. Anchor ids are
/// namespaced per document (`cite:<document>:<key>`) so the same key in two chapters of a combined
/// book never collides; the shared counter also numbers footnote documents.
fn resolve_citations(blocks: &mut Vec<BlockNode>, slugs: &mut SlugGenerator, typo: &Typography, code_font: &str, next_diagram_id: &mut usize) {
    let mut entries: HashMap<String, Vec<InlineNode>> = HashMap::new();
    blocks.retain_mut(|block| match block {
        BlockNode::Reference { key, content } => {
            entries.insert(std::mem::take(key), std::mem::take(content));
            false
        }
        _ => true,
    });
    if entries.is_empty() {
        return;
    }
    // Claim a slot in the document-wide id counter only once entries actually survive, so a
    // citations-less chapter doesn't shift numbering for the next one.
    let document = *next_diagram_id;
    *next_diagram_id += 1;
    let mut cited: Vec<String> = Vec::new();
    for_each_inlines(blocks, &mut |inlines| {
        // pulldown-cmark emits the "[" of an unresolved link as its own text event; rejoin
        // same-styled neighbours so "[@key]" is one run.
        inlines.dedup_by(|b, a| {
            let same = a.link_target.is_none() && b.link_target.is_none() && a.style == b.style;
            if same {
                a.text.push_str(&b.text);
            }
            same
        });
        splice_matches(inlines, code_font, '[', &mut |text, from| {
            let mut search = from;
            while let Some(start) = text[search..].find("[@").map(|i| i + search) {
                let Some(end) = text[start..].find(']').map(|i| i + start + 1) else { break };
                search = end;
                let keys: Vec<&str> = text[start + 1..end - 1].split(';').map(|k| k.trim().trim_start_matches('@')).collect();
                if let Some(unknown) = keys.iter().find(|k| !entries.contains_key(**k)) {
                    eprintln!("warning: citation of unknown reference `{unknown}`");
                    continue;
                }
                let mut numbers: Vec<usize> = keys
                    .iter()
                    .map(|k| match cited.iter().position(|c| c == k) {
                        Some(i) => i + 1,
                        None => {
                            cited.push(k.to_string());
                            cited.len()
                        }
                    })
                    .collect();
                numbers.sort_unstable();
                numbers.dedup();
                // Each number links to its bibliography entry.
                let mut pieces = vec![("[".to_string(), None)];
                for (i, n) in numbers.iter().enumerate() {
                    if i > 0 {
                        pieces.push((", ".to_string(), None));
                    }
                    pieces.push((n.to_string(), Some(LinkTarget::InternalAnchor(format!("cite:{document}:{}", cited[n - 1])))));
                }
                pieces.push(("]".to_string(), None));
                return Some((start, end, pieces));
            }
            None
        });
    });
    if cited.is_empty() {
        return;
    }
    let h = typo.heading.resolve(2);
    let heading_style =
        TextStyle { bold: h.bold, italic: false, strikethrough: false, size: h.size_pt, color: h.color.0, font_family: h.font_family.as_str().into() };
    blocks.push(BlockNode::Heading {
        level: 2,
        id: slugs.generate("References"),
        content: vec![InlineNode { text: "References".to_string(), style: heading_style, link_target: None }],
    });
    for (i, key) in cited.iter().enumerate() {
        let mut content = entries.remove(key).expect("cited keys were checked against entries");
        if let Some(first) = content.first().cloned() {
            content.insert(0, InlineNode { text: format!("[{}] ", i + 1), link_target: None, ..first });
        }
        blocks.push(BlockNode::Anchored { id: format!("cite:{document}:{key}"), content });
    }
}

/// Numbers footnotes in reference order and moves their definitions to the end of the document.
/// Each reference run becomes its superscript number carrying a document-unique
/// `LinkTarget::Footnote` id (unique so a book's chapters, each restarting at 1, never collide).
/// A definition nothing cites is dropped. A reference with no definition is restored to its
/// literal `[^label]` text, unlinked, with a warning instead of numbering into nothing.
fn resolve_footnotes(blocks: &mut Vec<BlockNode>, next_diagram_id: &mut usize) {
    let mut definitions: Vec<(String, Vec<InlineNode>)> = Vec::new();
    blocks.retain_mut(|block| match block {
        BlockNode::FootnoteDefinition { id, content } => {
            definitions.push((std::mem::take(id), std::mem::take(content)));
            false
        }
        _ => true,
    });
    if definitions.is_empty() {
        return;
    }
    let document = *next_diagram_id;
    *next_diagram_id += 1;
    let mut cited: Vec<String> = Vec::new();
    for_each_inlines(blocks, &mut |inlines| {
        for node in inlines {
            let Some(LinkTarget::Footnote(label)) = &node.link_target else { continue };
            if !definitions.iter().any(|(l, _)| l == label) {
                eprintln!("warning: footnote reference `[^{label}]` has no definition");
                node.text = format!("[^{label}]");
                node.link_target = None;
                continue;
            }
            let number = match cited.iter().position(|l| l == label) {
                Some(i) => i + 1,
                None => {
                    cited.push(label.clone());
                    cited.len()
                }
            };
            node.text = superscript(number);
            node.link_target = Some(LinkTarget::Footnote(format!("footnote-{document}-{number}")));
        }
    });
    for (i, label) in cited.iter().enumerate() {
        if let Some((_, content)) = definitions.iter_mut().find(|(l, _)| l == label) {
            let mut content = std::mem::take(content);
            if let Some(mut mark) = content.first().cloned() {
                mark.text = format!("{} ", superscript(i + 1));
                mark.link_target = None;
                content.insert(0, mark);
            }
            blocks.push(BlockNode::FootnoteDefinition { id: format!("footnote-{document}-{}", i + 1), content });
        }
    }
}

fn superscript(number: usize) -> String {
    number.to_string().chars().map(|d| ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'][d.to_digit(10).unwrap() as usize]).collect()
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
    TextStyle {
        bold: false,
        italic: false,
        strikethrough: false,
        size,
        color: DEFAULT_COLOR,
        font_family: sardown_style::HeadingStyle::default().font_family.into(),
    }
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
