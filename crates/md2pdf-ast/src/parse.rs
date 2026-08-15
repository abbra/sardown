use crate::{BlockNode, ColumnAlignment, HighlightedToken, ImageSource, InlineNode, LinkTarget, SlugGenerator, TextStyle};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const DEFAULT_BODY_SIZE: f32 = 12.0;
const HEADING_SIZES: [f32; 6] = [28.0, 22.0, 18.0, 16.0, 14.0, 12.0];
const DEFAULT_COLOR: [u8; 3] = [0, 0, 0];

struct InlineBuilder {
    runs: Vec<InlineNode>,
    bold_depth: u32,
    italic_depth: u32,
    link_target: Option<LinkTarget>,
    base_size: f32,
}

impl InlineBuilder {
    fn new(base_size: f32) -> Self {
        Self { runs: Vec::new(), bold_depth: 0, italic_depth: 0, link_target: None, base_size }
    }

    fn push_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.runs.push(InlineNode {
            text,
            style: TextStyle {
                bold: self.bold_depth > 0,
                italic: self.italic_depth > 0,
                size: self.base_size,
                color: DEFAULT_COLOR,
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
        Event::Code(text) => builder.push_text(text.into_string()),
        Event::Start(Tag::Strong) => builder.bold_depth += 1,
        Event::End(TagEnd::Strong) => builder.bold_depth = builder.bold_depth.saturating_sub(1),
        Event::Start(Tag::Emphasis) => builder.italic_depth += 1,
        Event::End(TagEnd::Emphasis) => builder.italic_depth = builder.italic_depth.saturating_sub(1),
        Event::Start(Tag::Link { dest_url, .. }) => {
            builder.link_target = Some(link_target_from_url(&dest_url));
        }
        Event::End(TagEnd::Link) => builder.link_target = None,
        Event::SoftBreak | Event::HardBreak => builder.push_text(" ".to_string()),
        _ => {}
    }
}

fn lower_inline_events<'a, I: Iterator<Item = Event<'a>>>(
    events: &mut I,
    end_tag: TagEnd,
    base_size: f32,
) -> Vec<InlineNode> {
    let mut builder = InlineBuilder::new(base_size);
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
    } else {
        ImageSource::Embedded(std::path::PathBuf::from(url))
    }
}

fn lower_block_events<'a, I: Iterator<Item = Event<'a>>>(
    parser: &mut I,
    end_tag: TagEnd,
    slugs: &mut SlugGenerator,
) -> Vec<BlockNode> {
    let mut blocks = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(tag) if tag == end_tag => break,
            Event::Start(Tag::Heading { level, .. }) => {
                let size = heading_size(level);
                let content = lower_inline_events(parser, TagEnd::Heading(level), size);
                let text: String = content.iter().map(|n| n.text.as_str()).collect::<Vec<_>>().join("");
                let id = slugs.generate(&text);
                blocks.push(BlockNode::Heading { level: heading_level_u8(level), id, content });
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
                        let mut builder = InlineBuilder::new(DEFAULT_BODY_SIZE);
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
                blocks.push(BlockNode::CodeBlock {
                    language,
                    tokens: vec![HighlightedToken { text: raw, color: DEFAULT_COLOR }],
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let content = lower_block_events(parser, TagEnd::BlockQuote(None), slugs);
                blocks.push(BlockNode::Blockquote { content });
            }
            Event::Rule => blocks.push(BlockNode::ThematicBreak),
            Event::Start(Tag::List(start)) => {
                let ordered = start.is_some();
                let mut items = Vec::new();
                while let Some(event) = parser.next() {
                    match event {
                        Event::Start(Tag::Item) => {
                            items.push(lower_block_events(parser, TagEnd::Item, slugs));
                        }
                        Event::End(TagEnd::List(_)) => break,
                        _ => {}
                    }
                }
                blocks.push(BlockNode::List { ordered, items });
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
                            headers = collect_table_cells(parser, TagEnd::TableHead);
                        }
                        Event::Start(Tag::TableRow) => {
                            let mut row = Vec::new();
                            while let Some(event) = parser.next() {
                                match event {
                                    Event::Start(Tag::TableCell) => {
                                        row.extend(lower_inline_events(parser, TagEnd::TableCell, DEFAULT_BODY_SIZE));
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
    parser: &mut I,
    end_tag: TagEnd,
) -> Vec<InlineNode> {
    let mut cells = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                cells.extend(lower_inline_events(parser, TagEnd::TableCell, DEFAULT_BODY_SIZE));
            }
            Event::End(tag) if tag == end_tag => break,
            _ => {}
        }
    }
    cells
}

fn collect_plain_text_until<'a, I: Iterator<Item = Event<'a>>>(
    parser: &mut I,
    end_tag: TagEnd,
) -> String {
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
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut parser = Parser::new_ext(markdown, options);
    let mut slugs = SlugGenerator::new();
    // TagEnd::Item is never opened at the top level, so it never matches; used only as a
    // sentinel that can't legitimately occur, meaning we consume until the iterator is exhausted.
    lower_block_events(&mut parser, TagEnd::Item, &mut slugs)
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

fn heading_size(level: HeadingLevel) -> f32 {
    HEADING_SIZES[(heading_level_u8(level) - 1) as usize]
}
