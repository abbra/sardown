use crate::{
    shape_paragraph, shape_rich_paragraph, AnchorPosition, AnchorTable, ImageTable, PageContext, PageGeometry, PathCommand, PositionedElement, PositionedPage,
    Rect, StrokeStyle,
};
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;
use md2pdf_enrich::DiagramTable;

const PT_PER_MM: f32 = 2.834645669;
const LINE_SPACING_PT: f32 = 4.0; // gap after each block

/// `style` is read fresh on every call into `render_block` -- but only *some* of its fields
/// actually feed rendered output that way. `Heading`/`Paragraph`/`Table` text size and color come
/// from each `InlineNode`'s own already-baked `TextStyle`, set once when the AST was originally
/// parsed, and are never re-read from `style` here no matter what it contains. The fields that
/// genuinely are read fresh, right here in this file, are: `typography.alignment` (heading/
/// paragraph alignment), `typography.body_size_pt`/`.body_color` (list markers, via
/// `marker_inline_node`), `table.text_size_pt` (row-height/padding math), `code_block`'s font
/// sizes (via `CodeBlockStyle::resolve`), and `heading.levels.*.underline_color`/
/// `.underline_width_pt` (via `HeadingStyle::resolve`).
///
/// This split matters most to `md2pdf-slides`, which swaps in a per-slide `Stylesheet` to make a
/// layout override or an auto-shrink scale step visible: doing that for a baked-in field (body/
/// heading/table-cell text size or color) requires directly mutating the AST's own `InlineNode`s
/// instead (see `md2pdf_slides::rescale_slide_content`'s doc comment for the full mechanism). A
/// new `SlideLayoutStyle` field that's meant to affect rendered text must be wired into whichever
/// of the two mechanisms actually owns the underlying `Stylesheet` field -- wiring it into the
/// wrong one (or only `build_slide_stylesheet`, when the field is actually baked-in) silently
/// does nothing.
struct Cursor<'a> {
    y: f32,
    page_height_pt: f32,
    content_width_pt: f32,
    pages: Vec<PositionedPage>,
    current: Vec<PositionedElement>,
    page_number: usize,
    anchors: AnchorTable,
    current_h1: Option<String>,
    current_h2: Option<String>,
    chapter_opener_pending: bool,
    page_contexts: Vec<PageContext>,
    style: &'a md2pdf_style::Stylesheet,
}

impl<'a> Cursor<'a> {
    fn new(geometry: &PageGeometry, style: &'a md2pdf_style::Stylesheet) -> Self {
        let margin_pt = geometry.margin_mm * PT_PER_MM;
        Self {
            y: margin_pt,
            page_height_pt: geometry.page_height_mm * PT_PER_MM - margin_pt, // bottom boundary
            content_width_pt: geometry.page_width_mm * PT_PER_MM - geometry.horizontal_margin_budget_mm() * PT_PER_MM,
            pages: Vec::new(),
            current: Vec::new(),
            page_number: 0,
            anchors: AnchorTable::new(),
            current_h1: None,
            current_h2: None,
            chapter_opener_pending: false,
            page_contexts: Vec::new(),
            style,
        }
    }

    /// Builds a Cursor for laying out one `::columns` column in isolation: an unbounded page
    /// height so `break_page`'s own height check can never fire (see the `BlockNode::Columns`
    /// arm's own doc comment for why), starting at `y = 0.0` in the column's own local coordinate
    /// space rather than a page margin. `current_h1`/`current_h2` are inherited from the parent
    /// cursor (a column never starts a new chapter of its own).
    fn isolated(content_width_pt: f32, style: &'a md2pdf_style::Stylesheet, current_h1: Option<String>, current_h2: Option<String>) -> Self {
        Self {
            y: 0.0,
            page_height_pt: f32::MAX,
            content_width_pt,
            pages: Vec::new(),
            current: Vec::new(),
            page_number: 0,
            anchors: AnchorTable::new(),
            current_h1,
            current_h2,
            chapter_opener_pending: false,
            page_contexts: Vec::new(),
            style,
        }
    }

    fn remaining_height(&self) -> f32 {
        self.page_height_pt - self.y
    }

    fn snapshot_page_context(&mut self) {
        self.page_contexts.push(PageContext {
            current_h1: self.current_h1.clone(),
            current_h2: self.current_h2.clone(),
            is_chapter_opener: std::mem::take(&mut self.chapter_opener_pending),
            suppress_header: false,
            suppress_footer: false,
        });
    }

    fn break_page(&mut self, margin_pt: f32) {
        let elements = std::mem::take(&mut self.current);
        self.pages.push(PositionedPage { page_number: self.page_number, elements });
        self.snapshot_page_context();
        self.page_number += 1;
        self.y = margin_pt;
    }

    fn finish(mut self) -> (Vec<PositionedPage>, AnchorTable, Vec<PageContext>) {
        if !self.current.is_empty() || self.pages.is_empty() {
            let elements = std::mem::take(&mut self.current);
            self.pages.push(PositionedPage { page_number: self.page_number, elements });
            self.snapshot_page_context();
        }
        (self.pages, self.anchors, self.page_contexts)
    }
}

/// Estimated block height, in points, before shaping — used only to decide whether the
/// block's first line fits; exact height comes from the shaped elements themselves once placed.
pub(crate) fn estimate_line_height(size: f32) -> f32 {
    size * 1.4 + LINE_SPACING_PT
}

/// Shapes and places `content` (a paragraph, heading, table cell, or synthetic code-token list),
/// keeping multiple styled runs (bold/italic/links, syntax-highlighted tokens) flowing on the
/// same visual line instead of giving each its own line, and emitting a `LinkAnnotation` for
/// every run whose source `InlineNode` carries a `link_target`.
///
/// `shape_rich_paragraph` emits one `ShapedRun` per *span* — every color/style/link change starts
/// a new one, even mid-line. Runs sharing the same pre-placement `y` (cosmic-text's `run.line_y`)
/// came from the same visual line and must be placed at one shared final `y` with a single cursor
/// advance, not one advance per run (placing each run independently would advance the cursor
/// once per span instead of once per line).
fn to_cosmic_align(alignment: md2pdf_style::TextAlignment) -> cosmic_text::Align {
    match alignment {
        md2pdf_style::TextAlignment::Left => cosmic_text::Align::Left,
        md2pdf_style::TextAlignment::Right => cosmic_text::Align::Right,
        md2pdf_style::TextAlignment::Center => cosmic_text::Align::Center,
        md2pdf_style::TextAlignment::Justify => cosmic_text::Align::Justified,
    }
}

/// Returns `(leftmost_x, rightmost_x)` in points -- the actual rendered horizontal extent of the
/// widest line placed. Used by the `Heading` arm to draw an underline hugging the heading's own
/// text (both its width *and* its actual start position, which shifts under `Align::Center`/
/// `Align::Right`) rather than assuming it always starts at `margin_pt + indent_pt`.
fn place_inline_content(
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    max_width_pt: f32,
    content: &[md2pdf_ast::InlineNode],
    align: cosmic_text::Align,
    hyphenator: Option<&crate::Hyphenator>,
    font_system: &mut FontSystem,
) -> (f32, f32) {
    let hyphenated;
    let content = match hyphenator {
        Some(h) => {
            hyphenated = crate::insert_hyphenation_breaks(content, h, max_width_pt, font_system);
            hyphenated.as_slice()
        }
        None => content,
    };
    let shaped = shape_rich_paragraph(font_system, content, max_width_pt, align);
    let mut iter = shaped.into_iter().peekable();

    let mut content_start_y = cursor.y;
    let mut first_line_y: Option<f32> = None;
    let line_start_x = margin_pt + indent_pt;
    let mut min_start_x = f32::MAX;
    let mut max_end_x = line_start_x;

    while let Some(first) = iter.next() {
        let line_y = match &first.element {
            PositionedElement::TextRun { y, .. } => *y,
            _ => unreachable!("shape_rich_paragraph only produces TextRun elements"),
        };
        let baseline_line_y = *first_line_y.get_or_insert(line_y);

        let mut line_height = match &first.element {
            PositionedElement::TextRun { size, .. } => estimate_line_height(*size),
            _ => unreachable!(),
        };
        let mut group = vec![first];
        while let Some(next) = iter.peek() {
            let same_line = matches!(&next.element, PositionedElement::TextRun { y, .. } if *y == line_y);
            if !same_line {
                break;
            }
            let next = iter.next().unwrap();
            if let PositionedElement::TextRun { size, .. } = &next.element {
                line_height = line_height.max(estimate_line_height(*size));
            }
            group.push(next);
        }

        let mut placed_y = content_start_y + (line_y - baseline_line_y);
        if cursor.page_height_pt - placed_y < line_height && !cursor.current.is_empty() {
            cursor.break_page(margin_pt);
            content_start_y = cursor.y;
            first_line_y = Some(line_y);
            placed_y = content_start_y;
        }

        for shaped_run in group {
            let source_style = &content[shaped_run.source_index].style;
            let link_target = content[shaped_run.source_index].link_target.clone();
            let strikethrough = source_style.strikethrough;
            let strikethrough_color = source_style.color;
            let mut element = shaped_run.element;
            let (rect, strike_line) = match &mut element {
                PositionedElement::TextRun { x, y, glyphs, size, .. } => {
                    *x += margin_pt + indent_pt;
                    *y = placed_y;
                    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
                    let rect = Rect { x: *x, y: placed_y - *size, width, height: *size * 1.2 };
                    min_start_x = min_start_x.min(rect.x);
                    max_end_x = max_end_x.max(rect.x + rect.width);
                    // A strikethrough line conventionally sits roughly through the x-height, above
                    // the baseline -- 0.3x the font size is a reasonable approximation without
                    // needing the font's own strikethrough-position metric (not exposed by the
                    // PositionedGlyph data this layer already works with).
                    let line_y = *y - *size * 0.3;
                    let line = strikethrough.then_some(([*x, line_y], [*x + width, line_y]));
                    (rect, line)
                }
                _ => unreachable!("shape_rich_paragraph only ever produces TextRun elements"),
            };
            cursor.current.push(element);
            if let Some(destination) = link_target {
                cursor.current.push(PositionedElement::LinkAnnotation { rect, destination });
            }
            if let Some((start, end)) = strike_line {
                cursor.current.push(PositionedElement::Path {
                    points: vec![PathCommand::MoveTo(start[0], start[1]), PathCommand::LineTo(end[0], end[1])],
                    fill: None,
                    stroke: Some(StrokeStyle { color: strikethrough_color, width: 1.0 }),
                });
            }
        }
        cursor.y = placed_y + line_height;
    }
    if min_start_x > max_end_x {
        (line_start_x, line_start_x) // no TextRun was ever placed (empty content)
    } else {
        (min_start_x, max_end_x)
    }
}

/// Shapes every cell in `row` (without placing anything) to find how tall the row will actually
/// render, so a page-break decision can be made *before* any of its cells are drawn. Deciding
/// per-cell instead (as rendering itself does) risks a page break landing between two cells of
/// the same row: every cell rendered after that point would reset to a `row_top_y` that belongs
/// to the wrong page, scattering the row's remaining cells across an unrelated part of the next
/// page (they'd land whatever the row's cursor-y offset was on the far side of the break).
fn measure_row_height(
    row: &[Vec<md2pdf_ast::InlineNode>],
    widths: &[f32],
    cell_padding_pt: f32,
    min_cell_wrap_width_pt: f32,
    min_row_height: f32,
    font_system: &mut FontSystem,
) -> f32 {
    let mut max_height = min_row_height;
    for (cell, width) in row.iter().zip(widths) {
        if cell.is_empty() {
            continue;
        }
        let cell_max_width_pt = (*width - cell_padding_pt).max(min_cell_wrap_width_pt);
        let shaped = shape_rich_paragraph(font_system, cell, cell_max_width_pt, cosmic_text::Align::Left);
        let mut ys: Vec<f32> = shaped
            .iter()
            .map(|r| match &r.element {
                PositionedElement::TextRun { y, .. } => *y,
                _ => unreachable!("shape_rich_paragraph only produces TextRun elements"),
            })
            .collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        if let (Some(&first), Some(&last)) = (ys.first(), ys.last()) {
            let size = cell[0].style.size;
            max_height = max_height.max((last - first) + estimate_line_height(size));
        }
    }
    max_height
}

/// A rough "how tall is this block's first line" estimate, used only to reserve enough gap
/// after a code block that its background doesn't reach up into the next block's ascender (see
/// the `BlockNode::CodeBlock` arm). Blocks without an obvious first-line size (tables, images,
/// diagrams, thematic breaks) fall back to `DEFAULT_BODY_SIZE_PT`, matching typical body text --
/// the only block kind whose ascent meaningfully exceeds that by enough to matter here is a
/// heading, since `HEADING_SIZES` (md2pdf-ast) run well above body text.
fn estimate_next_block_ascent_pt(next_block: Option<&BlockNode>) -> f32 {
    const DEFAULT_BODY_SIZE_PT: f32 = 12.0;
    let size = match next_block {
        Some(BlockNode::Heading { content, .. }) => content.first().map(|n| n.style.size).unwrap_or(DEFAULT_BODY_SIZE_PT),
        Some(BlockNode::Paragraph { content }) => content.first().map(|n| n.style.size).unwrap_or(DEFAULT_BODY_SIZE_PT),
        _ => DEFAULT_BODY_SIZE_PT,
    };
    size * 0.8
}

/// A list-item marker (bullet or "N.") in the document's own plain body style -- never inherits
/// whatever inline formatting (bold/italic/link/color) the item's own first word happens to have.
fn marker_inline_node(marker: &str, typography: &md2pdf_style::TypographyStyle) -> md2pdf_ast::InlineNode {
    md2pdf_ast::InlineNode {
        text: marker.to_string(),
        style: md2pdf_ast::TextStyle {
            bold: false,
            italic: false,
            strikethrough: false,
            size: typography.body_size_pt,
            color: typography.body_color.0,
            font_family: typography.font_family.clone(),
        },
        link_target: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    block: &BlockNode,
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    font_system: &mut FontSystem,
    images: &ImageTable,
    diagrams: &DiagramTable,
    hyphenator: Option<&crate::Hyphenator>,
    next_block: Option<&BlockNode>,
) {
    match block {
        BlockNode::Heading { level, content, id } => {
            let heading_size = content.first().map(|c| c.style.size).unwrap_or(12.0);
            // Skipped at the very top of a page/column, where extra leading whitespace isn't
            // wanted (e.g. a chapter's own title heading right after its PageBreak).
            if !cursor.current.is_empty() {
                cursor.y += heading_size * cursor.style.heading.space_before_factor;
            }
            let heading_h = estimate_line_height(heading_size);
            if cursor.remaining_height() < heading_h && !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }
            // A level-1 heading landing as the very first thing on a page -- whether because it
            // naturally opens there or because it just got pushed there by the fit check above --
            // marks that page as a "chapter opener" for header/footer suppression purposes.
            if cursor.current.is_empty() && *level == 1 {
                cursor.chapter_opener_pending = true;
            }
            let heading_text: String = content.iter().map(|n| n.text.as_str()).collect();
            if *level == 1 {
                cursor.current_h1 = Some(heading_text);
                cursor.current_h2 = None;
            } else if *level == 2 {
                cursor.current_h2 = Some(heading_text);
            }
            let anchor_y = cursor.y;
            let max_width_pt = cursor.content_width_pt - indent_pt;
            // Headings never justify/stretch -- Justify is forced to Left, matching the
            // pre-existing guarantee that headings stay left-aligned under a justified
            // stylesheet even though body paragraphs do stretch.
            let heading_align = match cursor.style.typography.alignment {
                md2pdf_style::TextAlignment::Justify => cosmic_text::Align::Left,
                other => to_cosmic_align(other),
            };
            let (heading_start_x, heading_end_x) =
                place_inline_content(cursor, margin_pt, indent_pt, max_width_pt, content, heading_align, None, font_system);
            let resolved_heading = cursor.style.heading.resolve(*level);
            if resolved_heading.underline_width_pt > 0.0 {
                // Approximates the last line's baseline plus a little clearance for descenders --
                // exact for the overwhelmingly common single-line heading; a multi-line heading
                // (rare) gets an underline positioned from this same estimate, not its own
                // measured last-line baseline.
                let underline_y = cursor.y - estimate_line_height(heading_size) + heading_size * 0.25;
                cursor.current.push(PositionedElement::Path {
                    points: vec![PathCommand::MoveTo(heading_start_x, underline_y), PathCommand::LineTo(heading_end_x, underline_y)],
                    fill: None,
                    stroke: Some(StrokeStyle { color: resolved_heading.underline_color.0, width: resolved_heading.underline_width_pt }),
                });
            }
            cursor.anchors.insert(id.clone(), AnchorPosition { page: cursor.page_number, x: margin_pt + indent_pt, y: anchor_y });
        }
        BlockNode::Paragraph { content } => {
            let max_width_pt = cursor.content_width_pt - indent_pt;
            place_inline_content(cursor, margin_pt, indent_pt, max_width_pt, content, to_cosmic_align(cursor.style.typography.alignment), hyphenator, font_system);
        }
        BlockNode::Blockquote { content } => {
            let start_y = cursor.y;
            let start_page = cursor.page_number;
            let child_indent_pt = indent_pt + cursor.style.blockquote.indent_pt;
            for (i, child) in content.iter().enumerate() {
                let child_next = content.get(i + 1).or(next_block);
                render_block(child, cursor, margin_pt, child_indent_pt, font_system, images, diagrams, hyphenator, child_next);
            }
            let end_y = cursor.y;
            let end_page = cursor.page_number;
            // `start_y`/`end_y` are cursor bookkeeping, not visual extents: `start_y` is the
            // first child's first line's *baseline* (its ascender reaches above that), and
            // `end_y` is the cursor position *after* the last line's full line height -- which
            // already includes the gap reserved for whatever block comes next. Left uncorrected
            // the border started visibly too low and ran down into the following block's own
            // text, same root cause as the CodeBlock background's ascender/gap padding.
            let pad = estimate_next_block_ascent_pt(content.first());
            let border_x = margin_pt + indent_pt + 4.0;
            let border_color = cursor.style.blockquote.border_color.0;
            let border_width = cursor.style.blockquote.border_width_pt;
            let border_segment = |top_y: f32, bottom_y: f32| PositionedElement::Path {
                points: vec![PathCommand::MoveTo(border_x, top_y), PathCommand::LineTo(border_x, bottom_y)],
                fill: None,
                stroke: Some(StrokeStyle { color: border_color, width: border_width }),
            };

            if end_page == start_page {
                cursor.current.push(border_segment(start_y - pad, end_y - pad - LINE_SPACING_PT));
            } else {
                // The blockquote's own content crossed one or more page breaks mid-render:
                // start_y/end_y are local to different pages' coordinate systems, so one line
                // spanning them would be meaningless (previously this drew a single huge,
                // nonsensical vertical line on the continuation page, cutting through whatever
                // unrelated content came after the blockquote). Draw one segment per page the
                // blockquote touches instead, matching the code block background's and table
                // grid's own handling of page-spanning content.
                cursor.pages[start_page].elements.push(border_segment(start_y - pad, cursor.page_height_pt));
                for page in (start_page + 1)..end_page {
                    cursor.pages[page].elements.push(border_segment(margin_pt - pad, cursor.page_height_pt));
                }
                cursor.current.push(border_segment(margin_pt - pad, end_y - pad - LINE_SPACING_PT));
            }
        }
        BlockNode::ThematicBreak => {
            let y = cursor.y + 6.0;
            let color = cursor.style.thematic_break.color.0;
            let width = cursor.style.thematic_break.width_pt;
            cursor.current.push(PositionedElement::Path {
                points: vec![PathCommand::MoveTo(margin_pt, y), PathCommand::LineTo(margin_pt + cursor.content_width_pt, y)],
                fill: None,
                stroke: Some(StrokeStyle { color, width }),
            });
            cursor.y += 12.0;
        }
        BlockNode::PageBreak => {
            if !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }
        }
        BlockNode::List { ordered, start, items } => {
            let child_indent_pt = indent_pt + cursor.style.list.indent_pt;
            for (item_i, item) in items.iter().enumerate() {
                let marker_text = if *ordered { format!("{}.  ", start.unwrap_or(1) + item_i as u64) } else { "\u{2022}  ".to_string() };
                for (child_i, child) in item.iter().enumerate() {
                    let child_next = item.get(child_i + 1).or_else(|| items.get(item_i + 1).and_then(|next_item| next_item.first())).or(next_block);
                    // The marker is prepended into the first child's own text, rather than drawn
                    // as a separate gutter element, so it automatically inherits the paragraph's
                    // own page-break handling for free -- see the design note on
                    // BlockNode::List for the hanging-indent trade-off this implies. Only a
                    // Paragraph first child gets one; a nested list/image/code block as the very
                    // first thing in an item has no natural place to attach one without a
                    // separate, page-break-synchronized gutter element, and is rare enough in
                    // real Markdown not to be worth that complexity.
                    if child_i == 0 {
                        if let BlockNode::Paragraph { content } = child {
                            let mut marked_content = vec![marker_inline_node(&marker_text, &cursor.style.typography)];
                            marked_content.extend(content.iter().cloned());
                            let marked = BlockNode::Paragraph { content: marked_content };
                            render_block(&marked, cursor, margin_pt, child_indent_pt, font_system, images, diagrams, hyphenator, child_next);
                            continue;
                        }
                    }
                    render_block(child, cursor, margin_pt, child_indent_pt, font_system, images, diagrams, hyphenator, child_next);
                }
            }
        }
        BlockNode::CodeBlock { language, tokens } => {
            let resolved = cursor.style.code_block.resolve(language.as_deref());
            let code_font_size_pt = resolved.font_size_pt;
            let code_background = resolved.background.0;
            let label_style = cursor.style.code_block.label_style;
            // Header bar: drawn (background, then its own label text, in that paint order) as a
            // fixed-height strip *before* any code content -- using plain sequential pushes here,
            // and capturing `background_insert_at` (for the code's own background, below) only
            // after these pushes, keeps this feature from needing any shared insertion-index
            // arithmetic with the code block's own background insertion further down.
            if label_style == md2pdf_style::LabelStyle::HeaderBar {
                let header_bar_height_pt = code_font_size_pt + 8.0;
                if cursor.remaining_height() < header_bar_height_pt + estimate_line_height(code_font_size_pt) && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                let header_bar_top_y = cursor.y;
                let header_bar_bottom_y = header_bar_top_y + header_bar_height_pt;
                cursor.current.push(PositionedElement::Path {
                    points: vec![
                        PathCommand::MoveTo(margin_pt + indent_pt, header_bar_top_y),
                        PathCommand::LineTo(margin_pt + cursor.content_width_pt, header_bar_top_y),
                        PathCommand::LineTo(margin_pt + cursor.content_width_pt, header_bar_bottom_y),
                        PathCommand::LineTo(margin_pt + indent_pt, header_bar_bottom_y),
                        PathCommand::Close,
                    ],
                    fill: Some(resolved.label_background.0),
                    stroke: None,
                });
                let label_node = md2pdf_ast::InlineNode {
                    text: resolved.label.clone(),
                    style: md2pdf_ast::TextStyle {
                        bold: false,
                        italic: false,
                        strikethrough: false,
                        size: code_font_size_pt,
                        color: resolved.label_color.0,
                        font_family: resolved.font_family.clone(),
                    },
                    link_target: None,
                };
                let mut label_elements = shape_paragraph(font_system, std::slice::from_ref(&label_node), cursor.content_width_pt);
                if let Some(PositionedElement::TextRun { x, y, .. }) = label_elements.first_mut() {
                    *x = margin_pt + indent_pt + 6.0;
                    *y = header_bar_bottom_y - 4.0;
                }
                if let Some(label_run) = label_elements.into_iter().next() {
                    cursor.current.push(label_run);
                }
                cursor.y = header_bar_bottom_y;
            }

            let start_y = cursor.y;
            let start_page = cursor.page_number;
            let background_insert_at = cursor.current.len();
            let mut combined: Vec<md2pdf_ast::InlineNode> = Vec::with_capacity(tokens.len() + 1);
            if cursor.style.code_block.label_style == md2pdf_style::LabelStyle::Inline {
                combined.push(md2pdf_ast::InlineNode {
                    text: format!("{}\n", resolved.label),
                    style: md2pdf_ast::TextStyle {
                        bold: false,
                        italic: false,
                        strikethrough: false,
                        size: code_font_size_pt,
                        color: resolved.label_color.0,
                        font_family: resolved.font_family.clone(),
                    },
                    link_target: None,
                });
            }
            combined.extend(tokens.iter().map(|t| md2pdf_ast::InlineNode {
                text: t.text.clone(),
                style: md2pdf_ast::TextStyle {
                    bold: false,
                    italic: false,
                    strikethrough: false,
                    size: code_font_size_pt,
                    color: t.color,
                    font_family: resolved.font_family.clone(),
                },
                link_target: None,
            }));
            // One rich-shaping call over all tokens so tokens on the same source line (e.g.
            // "fn " and "main" as separate syntect tokens) flow together on one visual line,
            // each keeping its own color; line breaks come from the embedded `\n` characters
            // syntect leaves at the end of each source line's tokens.
            let code_indent_pt = indent_pt + 8.0;
            let max_width_pt = cursor.content_width_pt - code_indent_pt;
            place_inline_content(cursor, margin_pt, code_indent_pt, max_width_pt, &combined, cosmic_text::Align::Left, None, font_system);
            let end_y = cursor.y;
            let end_page = cursor.page_number;
            let content_width_pt = cursor.content_width_pt;
            let page_height_pt = cursor.page_height_pt;

            // `PositionedElement::TextRun::y` is a *baseline*, not a glyph top: `start_y`/
            // `margin_pt` (a page's first line) mark where a line's baseline sits, so a
            // background meant to enclose the glyphs needs to reach up above that baseline by
            // roughly the font's ascent, not by a small fixed nudge — a flat 4pt pad left the
            // tops of ascenders (e.g. "P", "T") poking out above the gray box. 0.8x the code
            // font's size approximates typical sans-serif ascent; the bottom gets a smaller pad
            // for descenders.
            let normal_top_pad_pt = code_font_size_pt * 0.8;
            let start_top_pad_pt = if label_style == md2pdf_style::LabelStyle::Corner {
                code_font_size_pt * 1.8 // enough room for the corner badge to sit inside the enlarged pad
            } else {
                normal_top_pad_pt
            };
            // Must stay under LINE_SPACING_PT (the gap the layout loop inserts after every
            // block) or the box bleeds into whatever follows the code block.
            const BOTTOM_PAD_PT: f32 = 2.0;
            // `end_y` (== cursor.y right after placing the block) is where a *next* line would
            // start -- baseline + this line height, which already bakes in the trailing
            // inter-block gap -- not the last line's own visual bottom. Subtracting it back out
            // before adding the small descender pad avoids the background swallowing a whole
            // extra line's height below the actual last line.
            let last_line_baseline = end_y - estimate_line_height(code_font_size_pt);

            let background_rect = |top_y: f32, bottom_y: f32| PositionedElement::Path {
                points: vec![
                    PathCommand::MoveTo(margin_pt + indent_pt, top_y),
                    PathCommand::LineTo(margin_pt + content_width_pt, top_y),
                    PathCommand::LineTo(margin_pt + content_width_pt, bottom_y),
                    PathCommand::LineTo(margin_pt + indent_pt, bottom_y),
                    PathCommand::Close,
                ],
                fill: Some(code_background),
                stroke: None,
            };

            if end_page == start_page {
                // Inserted before the text elements just placed (rather than pushed after): the
                // PDF renderer paints elements in array order, so an opaque background pushed
                // *after* its own text would paint over that text instead of sitting behind it.
                cursor.current.insert(background_insert_at, background_rect(start_y - start_top_pad_pt, last_line_baseline + BOTTOM_PAD_PT));
            } else {
                // `place_inline_content` broke to one or more new pages mid-block: `start_y`/
                // `end_y` are local to different pages' coordinate systems, so one rectangle
                // spanning them would be meaningless (previously this drew a stray band on the
                // continuation page at whatever y start_y happened to be, unrelated to any of the
                // block's actual text). Draw one rectangle per page the block touches instead:
                // start page from its own top down to the page's bottom margin, any fully-spanned
                // middle pages the whole content height, and the final page from the top margin
                // down to end_y. Every page's first line restarts at that page's own margin with
                // the same baseline-vs-top-pad correction as the block's very first line -- except
                // a corner badge's enlarged pad, which only decorates the block's true start.
                cursor.pages[start_page].elements.insert(background_insert_at, background_rect(start_y - start_top_pad_pt, page_height_pt));
                for page in (start_page + 1)..end_page {
                    cursor.pages[page].elements.insert(0, background_rect(margin_pt - normal_top_pad_pt, page_height_pt));
                }
                cursor.current.insert(0, background_rect(margin_pt - normal_top_pad_pt, last_line_baseline + BOTTOM_PAD_PT));
            }

            // Corner badge: painted after the code's own background (a plain push always lands
            // after whatever's already on start_page, in both the single- and multi-page cases)
            // so it visually sits on top of it, overlapping the enlarged start-page top pad.
            // Always belongs to start_page, regardless of which page the code's own text ended
            // up spanning to.
            if label_style == md2pdf_style::LabelStyle::Corner {
                let badge_node = md2pdf_ast::InlineNode {
                    text: resolved.label.clone(),
                    style: md2pdf_ast::TextStyle {
                        bold: false,
                        italic: false,
                        strikethrough: false,
                        size: code_font_size_pt * 0.8,
                        color: resolved.label_color.0,
                        font_family: resolved.font_family.clone(),
                    },
                    link_target: None,
                };
                let badge_elements = shape_paragraph(font_system, std::slice::from_ref(&badge_node), content_width_pt);
                if let Some(PositionedElement::TextRun { glyphs, .. }) = badge_elements.first() {
                    let badge_text_width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
                    let badge_padding_x = 6.0;
                    let badge_width = badge_text_width + badge_padding_x * 2.0;
                    let badge_height = code_font_size_pt + 4.0;
                    let badge_right_x = margin_pt + content_width_pt;
                    let badge_left_x = badge_right_x - badge_width;
                    let badge_top_y = start_y - start_top_pad_pt;
                    let badge_bottom_y = badge_top_y + badge_height;
                    let badge_rect = PositionedElement::Path {
                        points: vec![
                            PathCommand::MoveTo(badge_left_x, badge_top_y),
                            PathCommand::LineTo(badge_right_x, badge_top_y),
                            PathCommand::LineTo(badge_right_x, badge_bottom_y),
                            PathCommand::LineTo(badge_left_x, badge_bottom_y),
                            PathCommand::Close,
                        ],
                        fill: Some(resolved.label_background.0),
                        stroke: None,
                    };
                    let mut badge_text = badge_elements.into_iter().next().unwrap();
                    if let PositionedElement::TextRun { x, y, .. } = &mut badge_text {
                        *x = badge_left_x + badge_padding_x;
                        *y = badge_bottom_y - 3.0;
                    }
                    if start_page == cursor.page_number {
                        cursor.current.push(badge_rect);
                        cursor.current.push(badge_text);
                    } else {
                        cursor.pages[start_page].elements.push(badge_rect);
                        cursor.pages[start_page].elements.push(badge_text);
                    }
                }
            }

            // The layout loop always adds a flat `LINE_SPACING_PT` gap after every block,
            // sized for typical body text -- nowhere near enough clearance if the next block is
            // a heading, whose much larger ascent would otherwise reach up into this block's
            // opaque background (observed: an H2 heading's top visibly sliced by the code
            // block's bottom edge). Reserve whatever extra the next block's actual ascent needs
            // beyond what LINE_SPACING_PT already provides.
            let next_ascent_pt = estimate_next_block_ascent_pt(next_block);
            let extra_gap_needed_pt = (next_ascent_pt + BOTTOM_PAD_PT - LINE_SPACING_PT).max(0.0);
            cursor.y += extra_gap_needed_pt;
        }
        BlockNode::Table { headers, rows, .. } => {
            let widths = crate::table::column_widths(headers, rows, cursor.content_width_pt - indent_pt, font_system);
            // Floor, not a fixed height: a row must be as tall as its tallest cell.
            // Real-world tables routinely have cells whose text wraps to multiple lines
            // (e.g. a "Description" column), which used to overflow this constant and
            // overlap the next row entirely.
            let min_row_height = cursor.style.table.min_row_height_pt;
            // Total horizontal padding reserved for a cell, split evenly left and right. Without
            // this, cell text started exactly at the column's left grid line -- flush against the
            // vertical divider, with no breathing room on either side.
            let cell_padding_pt = cursor.style.table.cell_padding_pt;
            let cell_padding_x_pt = cell_padding_pt / 2.0;
            // Not stylesheet-configurable -- an internal safety floor preventing a cell from
            // collapsing to zero width, not a visual style choice.
            const MIN_CELL_WRAP_WIDTH_PT: f32 = 10.0;
            // `PositionedElement::TextRun::y` is a baseline, and `cursor.y` after placing a row
            // is the *next* row's baseline -- not a safe boundary to draw a grid line on. Used
            // as-is, the header separator line landed exactly on row 1's baseline, cutting
            // through the middle of its text instead of sitting in the empty gap between rows.
            // Derived from the same `Stylesheet.table.text_size_pt` that `md2pdf-ast` gives every
            // table cell's own text -- previously two separate hardcoded constants kept in sync
            // only by a comment; now one real, enforced source of truth.
            let table_text_size_pt = cursor.style.table.text_size_pt;
            let table_top_pad_pt = table_text_size_pt * 0.8; // clears the first row's ascender
            let table_row_gap_adjust_pt = table_text_size_pt + 2.0; // baseline -> mid-gap-below-descender

            // A row is never split mid-cell across a page break: each row's height is measured
            // up front, and if it won't fit, the whole table breaks to a new page *before* any
            // of that row's cells are drawn. Placing cells one at a time and letting
            // `place_inline_content`'s own per-line break fire mid-row (the previous behavior)
            // corrupted every cell rendered after the break, since each resets to a `row_top_y`
            // belonging to the page the row started on.
            let header_height = measure_row_height(headers, &widths, cell_padding_pt, MIN_CELL_WRAP_WIDTH_PT, min_row_height, font_system);
            if cursor.remaining_height() < header_height && !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }

            let table_top_y = cursor.y;
            let mut col_x = margin_pt + indent_pt;
            let mut header_bottom_y = table_top_y + min_row_height;
            for (header, width) in headers.iter().zip(&widths) {
                // Reset before each cell: `place_inline_content` advances `cursor.y` as it lays
                // out lines, so without this reset every cell after the first in a row would
                // start below where the previous cell's text left off instead of at the row's top.
                cursor.y = table_top_y;
                // Capped at this column's own width (minus a little breathing room before the
                // grid line), not the page's remaining width: `place_inline_content`'s width
                // parameter is normally "wrap at the right margin," which is wrong for a cell —
                // it let long text bleed across into the next column's space instead of wrapping.
                let cell_max_width_pt = (width - cell_padding_pt).max(MIN_CELL_WRAP_WIDTH_PT);
                place_inline_content(cursor, margin_pt, col_x - margin_pt + cell_padding_x_pt, cell_max_width_pt, header, cosmic_text::Align::Left, None, font_system);
                header_bottom_y = header_bottom_y.max(cursor.y);
                col_x += width;
            }
            cursor.y = header_bottom_y;

            // A table can still span multiple pages (rows just never split mid-row), so grid
            // lines are tracked and drawn one page segment at a time instead of as one path
            // spanning coordinates from unrelated pages -- the same category of bug the code
            // block background had. `header_bottom_y` is `Some` only for the segment that
            // actually contains the header row.
            struct Segment {
                page: usize,
                top_y: f32,
                bottom_y: f32,
                header_bottom_y: Option<f32>,
            }
            let mut segments = vec![Segment {
                page: cursor.page_number,
                top_y: table_top_y - table_top_pad_pt,
                bottom_y: header_bottom_y - table_row_gap_adjust_pt,
                header_bottom_y: Some(header_bottom_y - table_row_gap_adjust_pt),
            }];

            for row in rows {
                let row_height = measure_row_height(row, &widths, cell_padding_pt, MIN_CELL_WRAP_WIDTH_PT, min_row_height, font_system);
                if cursor.remaining_height() < row_height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                    segments.push(Segment { page: cursor.page_number, top_y: cursor.y - table_top_pad_pt, bottom_y: cursor.y, header_bottom_y: None });
                }

                let row_top_y = cursor.y;
                let mut col_x = margin_pt + indent_pt;
                let mut row_bottom_y = row_top_y + min_row_height;
                for (cell, width) in row.iter().zip(&widths) {
                    cursor.y = row_top_y;
                    let cell_max_width_pt = (width - cell_padding_pt).max(MIN_CELL_WRAP_WIDTH_PT);
                    place_inline_content(cursor, margin_pt, col_x - margin_pt + cell_padding_x_pt, cell_max_width_pt, cell, cosmic_text::Align::Left, None, font_system);
                    row_bottom_y = row_bottom_y.max(cursor.y);
                    col_x += width;
                }
                cursor.y = row_bottom_y;
                segments.last_mut().unwrap().bottom_y = row_bottom_y - table_row_gap_adjust_pt;
            }

            for segment in segments {
                let path = crate::table::grid_path(margin_pt + indent_pt, segment.top_y, segment.bottom_y, segment.header_bottom_y, &widths);
                if segment.page == cursor.page_number {
                    cursor.current.push(path);
                } else {
                    cursor.pages[segment.page].elements.push(path);
                }
            }
        }
        BlockNode::Image { source: md2pdf_ast::ImageSource::Embedded(path), .. } => {
            let key = path.to_string_lossy().to_string();
            if let Some(decoded) = images.get(&key) {
                let max_width = cursor.content_width_pt - indent_pt;
                let max_height = cursor.page_height_pt - margin_pt;
                let (width, height) = fit_vector_graphic(decoded.width as f32, decoded.height as f32, max_width, max_height);
                if cursor.remaining_height() < height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                cursor.current.push(PositionedElement::RasterImage { x: margin_pt + indent_pt, y: cursor.y, width, height, image_id: key });
                cursor.y += height;
            } else if let Some(diagram) = diagrams.get(&key) {
                // An embedded .svg file (collect_svg_diagrams) rather than a raster image --
                // rendered through the exact same VectorGraphic path Mermaid diagrams use.
                let max_width = cursor.content_width_pt - indent_pt;
                let max_height = cursor.page_height_pt - margin_pt;
                let (width, height) = fit_vector_graphic(diagram.width, diagram.height, max_width, max_height);
                if cursor.remaining_height() < height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                cursor.current.push(PositionedElement::VectorGraphic {
                    x: margin_pt + indent_pt,
                    y: cursor.y,
                    width,
                    height,
                    diagram_id: key,
                });
                cursor.y += height;
            }
        }
        BlockNode::Image { source: md2pdf_ast::ImageSource::External(_), .. } => {} // skipped, see decode_images
        BlockNode::MermaidDiagram { id, .. } => {
            if let Some(diagram) = diagrams.get(id) {
                let max_width = cursor.content_width_pt - indent_pt;
                let max_height = cursor.page_height_pt - margin_pt;
                let (width, height) = fit_vector_graphic(diagram.width, diagram.height, max_width, max_height);
                if cursor.remaining_height() < height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                cursor.current.push(PositionedElement::VectorGraphic { x: margin_pt + indent_pt, y: cursor.y, width, height, diagram_id: id.clone() });
                cursor.y += height;
                // Same "next block's ascender pokes above a preceding block's hard bottom edge"
                // issue CodeBlock's own background has (see estimate_next_block_ascent_pt) --
                // a diagram has a crisp bottom border, so any following block whose first line's
                // ascender extends further than the flat LINE_SPACING_PT gap visibly punches
                // into the diagram instead of sitting cleanly below it.
                let next_ascent_pt = estimate_next_block_ascent_pt(next_block);
                cursor.y += (next_ascent_pt - LINE_SPACING_PT).max(0.0);
            }
        }
        BlockNode::Columns(columns) => {
            // Lays out each column against an *isolated* Cursor with an unbounded page height
            // (`f32::MAX`), so break_page's own `remaining_height() < needed` check can never
            // fire -- the column's entire content always renders as one continuous stream, which
            // `Cursor::finish()` always returns as exactly one PositionedPage. Reusing the real
            // render_block loop this way gets every existing block type (lists, code blocks,
            // images, links) working inside a column for free, with no new per-block-type code.
            // A `::columns` block is treated as atomic: it isn't page-broken internally here (see
            // the design spec's own "no mid-block page break" non-goal) -- if it doesn't fit in
            // the remaining space, it renders anyway rather than splitting a column's content
            // across two pages.
            let gap_pt = cursor.style.columns.gap_pt;
            let n = columns.len().max(1) as f32;
            let available_width_pt = cursor.content_width_pt - indent_pt;
            let column_width_pt = ((available_width_pt - gap_pt * (n - 1.0)) / n).max(1.0);
            let outer_y = cursor.y;
            let mut max_height_pt: f32 = 0.0;

            for (i, column_blocks) in columns.iter().enumerate() {
                let mut sub_cursor = Cursor::isolated(column_width_pt, cursor.style, cursor.current_h1.clone(), cursor.current_h2.clone());
                for (j, block) in column_blocks.iter().enumerate() {
                    render_block(block, &mut sub_cursor, 0.0, 0.0, font_system, images, diagrams, hyphenator, column_blocks.get(j + 1));
                    sub_cursor.y += LINE_SPACING_PT;
                }
                let column_height_pt = sub_cursor.y;
                let (sub_pages, sub_anchors, _) = sub_cursor.finish();
                let column_x_offset_pt = margin_pt + indent_pt + i as f32 * (column_width_pt + gap_pt);

                // A column is documented as always producing exactly one internal page (an
                // unbounded page height means break_page's height check can never fire) -- the
                // only way to violate that today would be a BlockNode::PageBreak inside a
                // column, which no current producer of PageBreak/group_columns combination can
                // create. Warn loudly rather than silently dropping the extra page(s) if that
                // ever changes.
                if sub_pages.len() > 1 {
                    eprintln!(
                        "warning: a `::columns` column produced {} internal pages; only the \
                         first is kept and the rest is dropped, since columns render as one \
                         atomic block",
                        sub_pages.len()
                    );
                }
                if let Some(page) = sub_pages.into_iter().next() {
                    for mut element in page.elements {
                        crate::shift_element(&mut element, column_x_offset_pt, outer_y);
                        cursor.current.push(element);
                    }
                }
                for (id, anchor) in sub_anchors {
                    cursor.anchors.insert(id, AnchorPosition { page: cursor.page_number, x: anchor.x + column_x_offset_pt, y: anchor.y + outer_y });
                }
                max_height_pt = max_height_pt.max(column_height_pt);
            }
            cursor.y = outer_y + max_height_pt;
        }
    }
}

/// Fits a vector graphic's natural (width, height) within `max_width`/`max_height`, preserving
/// aspect ratio: capped by width first, then -- since a diagram taller (relative to its width)
/// than a full page's content area would still overflow past the bottom margin even on a fresh
/// page -- re-capped by height, re-deriving width from that so the aspect ratio still holds.
/// Shared by Mermaid diagrams and embedded `.svg` images (both rendered as `VectorGraphic`) and
/// embedded raster images (`RasterImage`) -- despite the name, nothing here is vector-specific;
/// it's just where this logic was first introduced.
fn fit_vector_graphic(natural_width: f32, natural_height: f32, max_width: f32, max_height: f32) -> (f32, f32) {
    let aspect = natural_height / natural_width;
    let (mut width, mut height) = if natural_width > max_width { (max_width, max_width * aspect) } else { (natural_width, natural_height) };
    if height > max_height {
        height = max_height;
        width = max_height / aspect;
    }
    (width, height)
}

pub struct LayoutOutput {
    pub pages: Vec<PositionedPage>,
    pub images: ImageTable,
    /// The diagram table actually used during layout -- the caller's own `DiagramTable` (Mermaid
    /// diagrams, compiled before layout ever runs) extended with any embedded `.svg` images
    /// discovered by `collect_svg_diagrams` (which can only happen once `base_dir` is known,
    /// inside layout itself). Callers must render with *this* table, not the one they originally
    /// passed in -- rendering with the original would silently drop every embedded SVG image
    /// (they'd resolve to `None` in `render_pdf`'s `VectorGraphic` lookup and draw nothing, with
    /// no warning at all, since that lookup has no failure path).
    pub diagrams: DiagramTable,
    pub anchors: AnchorTable,
    pub page_contexts: Vec<PageContext>,
    pub page_width_pt: f32,
    pub page_height_pt: f32,
    pub toc_entries: Vec<crate::toc::TocEntry>,
}

pub fn layout(ast: &[BlockNode], geometry: &PageGeometry, font_system: &mut FontSystem, base_dir: &std::path::Path, diagrams: &DiagramTable) -> LayoutOutput {
    layout_impl(ast, geometry, font_system, base_dir, diagrams, &md2pdf_style::Stylesheet::default())
}

/// The real implementation behind `layout()`. Takes the full `Stylesheet` explicitly instead of
/// individual values so `layout_with_header_footer` can thread a real stylesheet's values
/// through, while `layout()` itself keeps its exact original signature and default behavior for
/// every existing caller.
pub fn layout_impl(
    ast: &[BlockNode],
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
    base_dir: &std::path::Path,
    diagrams: &DiagramTable,
    stylesheet: &md2pdf_style::Stylesheet,
) -> LayoutOutput {
    let images = crate::image::decode_images(ast, base_dir);
    // Embedded .svg images are collected separately from Mermaid-compiled diagrams (a different
    // crate, with no filesystem access) but rendered through the exact same VectorGraphic path --
    // merge them into one local table so render_block doesn't need to know the difference.
    let mut diagrams = diagrams.clone();
    diagrams.extend(crate::image::collect_svg_diagrams(ast, base_dir));
    let hyphenator = if stylesheet.typography.hyphenation { crate::Hyphenator::load(&stylesheet.typography.language) } else { None };
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let mut cursor = Cursor::new(geometry, stylesheet);
    for (i, block) in ast.iter().enumerate() {
        render_block(block, &mut cursor, margin_pt, 0.0, font_system, &images, &diagrams, hyphenator.as_ref(), ast.get(i + 1));
        cursor.y += LINE_SPACING_PT;
    }
    let (pages, anchors, page_contexts) = cursor.finish();
    LayoutOutput {
        pages,
        images,
        diagrams,
        anchors,
        page_contexts,
        page_width_pt: geometry.page_width_mm * PT_PER_MM,
        page_height_pt: geometry.page_height_mm * PT_PER_MM,
        toc_entries: Vec::new(),
    }
}
