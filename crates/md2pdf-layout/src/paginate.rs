use crate::{
    shape_rich_paragraph, AnchorPosition, AnchorTable, ImageTable, PageContext, PageGeometry,
    PathCommand, PositionedElement, PositionedPage, Rect, StrokeStyle,
};
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;
use md2pdf_enrich::DiagramTable;

const PT_PER_MM: f32 = 2.834645669;
const LINE_SPACING_PT: f32 = 4.0; // gap after each block
const BLOCKQUOTE_INDENT_PT: f32 = 18.0;
const LIST_INDENT_PT: f32 = 18.0;
const CODE_BLOCK_BG: [u8; 3] = [245, 245, 245];
// A heading needs more visual separation from whatever precedes it (a different, already-
// finished section) than from its own following content, or it reads as the tail of the wrong
// section. Scales with the heading's own size so bigger headings get proportionally more room.
const SPACE_BEFORE_HEADING_FACTOR: f32 = 0.8;

struct Cursor {
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
}

impl Cursor {
    fn new(geometry: &PageGeometry) -> Self {
        let margin_pt = geometry.margin_mm * PT_PER_MM;
        Self {
            y: margin_pt,
            page_height_pt: geometry.page_height_mm * PT_PER_MM - margin_pt, // bottom boundary
            content_width_pt: geometry.page_width_mm * PT_PER_MM - 2.0 * margin_pt,
            pages: Vec::new(),
            current: Vec::new(),
            page_number: 0,
            anchors: AnchorTable::new(),
            current_h1: None,
            current_h2: None,
            chapter_opener_pending: false,
            page_contexts: Vec::new(),
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
fn estimate_line_height(size: f32) -> f32 {
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
fn place_inline_content(
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    max_width_pt: f32,
    content: &[md2pdf_ast::InlineNode],
    font_system: &mut FontSystem,
) {
    let shaped = shape_rich_paragraph(font_system, content, max_width_pt);
    let mut iter = shaped.into_iter().peekable();

    let mut content_start_y = cursor.y;
    let mut first_line_y: Option<f32> = None;

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
            let link_target = content[shaped_run.source_index].link_target.clone();
            let mut element = shaped_run.element;
            let rect = match &mut element {
                PositionedElement::TextRun { x, y, glyphs, size, .. } => {
                    *x += margin_pt + indent_pt;
                    *y = placed_y;
                    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
                    Rect { x: *x, y: placed_y - *size, width, height: *size * 1.2 }
                }
                _ => unreachable!("shape_rich_paragraph only ever produces TextRun elements"),
            };
            cursor.current.push(element);
            if let Some(destination) = link_target {
                cursor.current.push(PositionedElement::LinkAnnotation { rect, destination });
            }
        }
        cursor.y = placed_y + line_height;
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
        let shaped = shape_rich_paragraph(font_system, cell, cell_max_width_pt);
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

fn render_block(
    block: &BlockNode,
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    font_system: &mut FontSystem,
    images: &ImageTable,
    diagrams: &DiagramTable,
    next_block: Option<&BlockNode>,
) {
    match block {
        BlockNode::Heading { level, content, id } => {
            let heading_size = content.first().map(|c| c.style.size).unwrap_or(12.0);
            // Skipped at the very top of a page/column, where extra leading whitespace isn't
            // wanted (e.g. a chapter's own title heading right after its PageBreak).
            if !cursor.current.is_empty() {
                cursor.y += heading_size * SPACE_BEFORE_HEADING_FACTOR;
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
            place_inline_content(cursor, margin_pt, indent_pt, max_width_pt, content, font_system);
            cursor.anchors.insert(
                id.clone(),
                AnchorPosition { page: cursor.page_number, x: margin_pt + indent_pt, y: anchor_y },
            );
        }
        BlockNode::Paragraph { content } => {
            let max_width_pt = cursor.content_width_pt - indent_pt;
            place_inline_content(cursor, margin_pt, indent_pt, max_width_pt, content, font_system);
        }
        BlockNode::Blockquote { content } => {
            let start_y = cursor.y;
            let start_page = cursor.page_number;
            for (i, child) in content.iter().enumerate() {
                let child_next = content.get(i + 1).or(next_block);
                render_block(child, cursor, margin_pt, indent_pt + BLOCKQUOTE_INDENT_PT, font_system, images, diagrams, child_next);
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
            let border_segment = |top_y: f32, bottom_y: f32| PositionedElement::Path {
                points: vec![PathCommand::MoveTo(border_x, top_y), PathCommand::LineTo(border_x, bottom_y)],
                fill: None,
                stroke: Some(StrokeStyle { color: [180, 180, 180], width: 2.0 }),
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
            cursor.current.push(PositionedElement::Path {
                points: vec![PathCommand::MoveTo(margin_pt, y), PathCommand::LineTo(margin_pt + cursor.content_width_pt, y)],
                fill: None,
                stroke: Some(StrokeStyle { color: [200, 200, 200], width: 1.0 }),
            });
            cursor.y += 12.0;
        }
        BlockNode::PageBreak => {
            if !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }
        }
        BlockNode::List { items, .. } => {
            for (item_i, item) in items.iter().enumerate() {
                for (child_i, child) in item.iter().enumerate() {
                    let child_next = item
                        .get(child_i + 1)
                        .or_else(|| items.get(item_i + 1).and_then(|next_item| next_item.first()))
                        .or(next_block);
                    render_block(child, cursor, margin_pt, indent_pt + LIST_INDENT_PT, font_system, images, diagrams, child_next);
                }
            }
        }
        BlockNode::CodeBlock { tokens, .. } => {
            let start_y = cursor.y;
            let start_page = cursor.page_number;
            let background_insert_at = cursor.current.len();
            let combined: Vec<md2pdf_ast::InlineNode> = tokens
                .iter()
                .map(|t| md2pdf_ast::InlineNode {
                    text: t.text.clone(),
                    style: md2pdf_ast::TextStyle { bold: false, italic: false, size: 10.0, color: t.color },
                    link_target: None,
                })
                .collect();
            // One rich-shaping call over all tokens so tokens on the same source line (e.g.
            // "fn " and "main" as separate syntect tokens) flow together on one visual line,
            // each keeping its own color; line breaks come from the embedded `\n` characters
            // syntect leaves at the end of each source line's tokens.
            let code_indent_pt = indent_pt + 8.0;
            let max_width_pt = cursor.content_width_pt - code_indent_pt;
            place_inline_content(cursor, margin_pt, code_indent_pt, max_width_pt, &combined, font_system);
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
            const CODE_FONT_SIZE_PT: f32 = 10.0;
            const TOP_PAD_PT: f32 = CODE_FONT_SIZE_PT * 0.8;
            // Must stay under LINE_SPACING_PT (the gap the layout loop inserts after every
            // block) or the box bleeds into whatever follows the code block.
            const BOTTOM_PAD_PT: f32 = 2.0;
            // `end_y` (== cursor.y right after placing the block) is where a *next* line would
            // start -- baseline + this line height, which already bakes in the trailing
            // inter-block gap -- not the last line's own visual bottom. Subtracting it back out
            // before adding the small descender pad avoids the background swallowing a whole
            // extra line's height below the actual last line.
            let last_line_baseline = end_y - estimate_line_height(CODE_FONT_SIZE_PT);

            let background_rect = |top_y: f32, bottom_y: f32| PositionedElement::Path {
                points: vec![
                    PathCommand::MoveTo(margin_pt + indent_pt, top_y),
                    PathCommand::LineTo(margin_pt + content_width_pt, top_y),
                    PathCommand::LineTo(margin_pt + content_width_pt, bottom_y),
                    PathCommand::LineTo(margin_pt + indent_pt, bottom_y),
                    PathCommand::Close,
                ],
                fill: Some(CODE_BLOCK_BG),
                stroke: None,
            };

            if end_page == start_page {
                // Inserted before the text elements just placed (rather than pushed after): the
                // PDF renderer paints elements in array order, so an opaque background pushed
                // *after* its own text would paint over that text instead of sitting behind it.
                cursor
                    .current
                    .insert(background_insert_at, background_rect(start_y - TOP_PAD_PT, last_line_baseline + BOTTOM_PAD_PT));
            } else {
                // `place_inline_content` broke to one or more new pages mid-block: `start_y`/
                // `end_y` are local to different pages' coordinate systems, so one rectangle
                // spanning them would be meaningless (previously this drew a stray band on the
                // continuation page at whatever y start_y happened to be, unrelated to any of the
                // block's actual text). Draw one rectangle per page the block touches instead:
                // start page from its own top down to the page's bottom margin, any fully-spanned
                // middle pages the whole content height, and the final page from the top margin
                // down to end_y. Every page's first line restarts at that page's own margin with
                // the same baseline-vs-top-pad correction as the block's very first line.
                cursor
                    .pages[start_page]
                    .elements
                    .insert(background_insert_at, background_rect(start_y - TOP_PAD_PT, page_height_pt));
                for page in (start_page + 1)..end_page {
                    cursor.pages[page].elements.insert(0, background_rect(margin_pt - TOP_PAD_PT, page_height_pt));
                }
                cursor.current.insert(0, background_rect(margin_pt - TOP_PAD_PT, last_line_baseline + BOTTOM_PAD_PT));
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
            const MIN_ROW_HEIGHT: f32 = 20.0;
            // Total horizontal padding reserved for a cell, split evenly left and right. Without
            // this, cell text started exactly at the column's left grid line -- flush against the
            // vertical divider, with no breathing room on either side.
            const CELL_PADDING_PT: f32 = 12.0;
            const CELL_PADDING_X_PT: f32 = CELL_PADDING_PT / 2.0;
            const MIN_CELL_WRAP_WIDTH_PT: f32 = 10.0;
            // `PositionedElement::TextRun::y` is a baseline, and `cursor.y` after placing a row
            // is the *next* row's baseline -- not a safe boundary to draw a grid line on. Used
            // as-is, the header separator line landed exactly on row 1's baseline, cutting
            // through the middle of its text instead of sitting in the empty gap between rows.
            // `md2pdf-ast::parse` gives all table content the same fixed size (its
            // `TABLE_CELL_SIZE`), so a single constant (rather than inspecting each cell's style)
            // is enough here, matching the code block background's approach. Must stay in sync
            // with md2pdf-ast's `TABLE_CELL_SIZE`.
            const TABLE_TEXT_SIZE_PT: f32 = 10.5;
            const TABLE_TOP_PAD_PT: f32 = TABLE_TEXT_SIZE_PT * 0.8; // clears the first row's ascender
            const TABLE_ROW_GAP_ADJUST_PT: f32 = TABLE_TEXT_SIZE_PT + 2.0; // baseline -> mid-gap-below-descender

            // A row is never split mid-cell across a page break: each row's height is measured
            // up front, and if it won't fit, the whole table breaks to a new page *before* any
            // of that row's cells are drawn. Placing cells one at a time and letting
            // `place_inline_content`'s own per-line break fire mid-row (the previous behavior)
            // corrupted every cell rendered after the break, since each resets to a `row_top_y`
            // belonging to the page the row started on.
            let header_height =
                measure_row_height(headers, &widths, CELL_PADDING_PT, MIN_CELL_WRAP_WIDTH_PT, MIN_ROW_HEIGHT, font_system);
            if cursor.remaining_height() < header_height && !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }

            let table_top_y = cursor.y;
            let mut col_x = margin_pt + indent_pt;
            let mut header_bottom_y = table_top_y + MIN_ROW_HEIGHT;
            for (header, width) in headers.iter().zip(&widths) {
                // Reset before each cell: `place_inline_content` advances `cursor.y` as it lays
                // out lines, so without this reset every cell after the first in a row would
                // start below where the previous cell's text left off instead of at the row's top.
                cursor.y = table_top_y;
                // Capped at this column's own width (minus a little breathing room before the
                // grid line), not the page's remaining width: `place_inline_content`'s width
                // parameter is normally "wrap at the right margin," which is wrong for a cell —
                // it let long text bleed across into the next column's space instead of wrapping.
                let cell_max_width_pt = (width - CELL_PADDING_PT).max(MIN_CELL_WRAP_WIDTH_PT);
                place_inline_content(cursor, margin_pt, col_x - margin_pt + CELL_PADDING_X_PT, cell_max_width_pt, header, font_system);
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
                top_y: table_top_y - TABLE_TOP_PAD_PT,
                bottom_y: header_bottom_y - TABLE_ROW_GAP_ADJUST_PT,
                header_bottom_y: Some(header_bottom_y - TABLE_ROW_GAP_ADJUST_PT),
            }];

            for row in rows {
                let row_height =
                    measure_row_height(row, &widths, CELL_PADDING_PT, MIN_CELL_WRAP_WIDTH_PT, MIN_ROW_HEIGHT, font_system);
                if cursor.remaining_height() < row_height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                    segments.push(Segment {
                        page: cursor.page_number,
                        top_y: cursor.y - TABLE_TOP_PAD_PT,
                        bottom_y: cursor.y,
                        header_bottom_y: None,
                    });
                }

                let row_top_y = cursor.y;
                let mut col_x = margin_pt + indent_pt;
                let mut row_bottom_y = row_top_y + MIN_ROW_HEIGHT;
                for (cell, width) in row.iter().zip(&widths) {
                    cursor.y = row_top_y;
                    let cell_max_width_pt = (width - CELL_PADDING_PT).max(MIN_CELL_WRAP_WIDTH_PT);
                    place_inline_content(cursor, margin_pt, col_x - margin_pt + CELL_PADDING_X_PT, cell_max_width_pt, cell, font_system);
                    row_bottom_y = row_bottom_y.max(cursor.y);
                    col_x += width;
                }
                cursor.y = row_bottom_y;
                segments.last_mut().unwrap().bottom_y = row_bottom_y - TABLE_ROW_GAP_ADJUST_PT;
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
                let aspect = decoded.height as f32 / decoded.width as f32;
                let (width, height) = if decoded.width as f32 > max_width {
                    (max_width, max_width * aspect)
                } else {
                    (decoded.width as f32, decoded.height as f32)
                };
                if cursor.remaining_height() < height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                cursor.current.push(PositionedElement::RasterImage { x: margin_pt + indent_pt, y: cursor.y, width, height, image_id: key });
                cursor.y += height;
            }
        }
        BlockNode::Image { source: md2pdf_ast::ImageSource::External(_), .. } => {} // skipped, see decode_images
        BlockNode::MermaidDiagram { id, .. } => {
            if let Some(diagram) = diagrams.get(id) {
                let max_width = cursor.content_width_pt - indent_pt;
                let aspect = diagram.height / diagram.width;
                let (mut width, mut height) = if diagram.width > max_width {
                    (max_width, max_width * aspect)
                } else {
                    (diagram.width, diagram.height)
                };
                // Fitting by width alone isn't enough: a diagram taller (relative to its width)
                // than a full page's content area would still overflow past the bottom margin
                // even on a fresh page -- breaking to a new page can't help when the diagram is
                // too big for ANY page, not just the current one. Cap by the full page height a
                // fresh page provides and re-derive width from that to keep the aspect ratio.
                let max_height = cursor.page_height_pt - margin_pt;
                if height > max_height {
                    height = max_height;
                    width = max_height / aspect;
                }
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
    }
}

pub struct LayoutOutput {
    pub pages: Vec<PositionedPage>,
    pub images: ImageTable,
    pub anchors: AnchorTable,
    pub page_contexts: Vec<PageContext>,
}

pub fn layout(
    ast: &[BlockNode],
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
    base_dir: &std::path::Path,
    diagrams: &DiagramTable,
) -> LayoutOutput {
    let images = crate::image::decode_images(ast, base_dir);
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let mut cursor = Cursor::new(geometry);
    for (i, block) in ast.iter().enumerate() {
        render_block(block, &mut cursor, margin_pt, 0.0, font_system, &images, diagrams, ast.get(i + 1));
        cursor.y += LINE_SPACING_PT;
    }
    let (pages, anchors, page_contexts) = cursor.finish();
    LayoutOutput { pages, images, anchors, page_contexts }
}
