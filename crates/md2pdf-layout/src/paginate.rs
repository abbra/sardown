use crate::{shape_paragraph, shape_rich_paragraph, ImageTable, PageGeometry, PathCommand, PositionedElement, PositionedPage, StrokeStyle};
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;
use md2pdf_enrich::DiagramTable;

const PT_PER_MM: f32 = 2.834645669;
const LINE_SPACING_PT: f32 = 4.0; // gap after each block
const BLOCKQUOTE_INDENT_PT: f32 = 18.0;
const LIST_INDENT_PT: f32 = 18.0;
const CODE_BLOCK_BG: [u8; 3] = [245, 245, 245];

struct Cursor {
    y: f32,
    page_height_pt: f32,
    content_width_pt: f32,
    pages: Vec<PositionedPage>,
    current: Vec<PositionedElement>,
    page_number: usize,
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
        }
    }

    fn remaining_height(&self) -> f32 {
        self.page_height_pt - self.y
    }

    fn break_page(&mut self, margin_pt: f32) {
        let elements = std::mem::take(&mut self.current);
        self.pages.push(PositionedPage { page_number: self.page_number, elements });
        self.page_number += 1;
        self.y = margin_pt;
    }

    fn finish(mut self) -> Vec<PositionedPage> {
        if !self.current.is_empty() || self.pages.is_empty() {
            self.pages.push(PositionedPage { page_number: self.page_number, elements: self.current });
        }
        self.pages
    }
}

/// Estimated block height, in points, before shaping — used only to decide whether the
/// block's first line fits; exact height comes from the shaped elements themselves once placed.
fn estimate_line_height(size: f32) -> f32 {
    size * 1.4 + LINE_SPACING_PT
}

fn place_text_run(cursor: &mut Cursor, margin_pt: f32, mut element: PositionedElement) {
    let element_height = if let PositionedElement::TextRun { size, .. } = &element {
        estimate_line_height(*size)
    } else {
        0.0
    };
    if cursor.remaining_height() < element_height && !cursor.current.is_empty() {
        cursor.break_page(margin_pt);
    }
    if let PositionedElement::TextRun { x, y, .. } = &mut element {
        *x += margin_pt;
        *y += cursor.y;
    }
    cursor.y += element_height;
    cursor.current.push(element);
}

fn place_inline_content(
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    content: &[md2pdf_ast::InlineNode],
    font_system: &mut FontSystem,
) {
    for mut element in shape_paragraph(font_system, content, cursor.content_width_pt - indent_pt) {
        if let PositionedElement::TextRun { x, .. } = &mut element {
            *x += indent_pt;
        }
        place_text_run(cursor, margin_pt, element);
    }
}

/// Like `place_inline_content`, but keeps multiple styled runs (e.g. syntax-highlighted tokens)
/// flowing on the same visual line instead of giving each its own line.
///
/// `shape_rich_paragraph` emits one `ShapedRun` per *span* — every color/style change starts a
/// new one, even mid-line. Runs sharing the same pre-placement `y` (cosmic-text's `run.line_y`)
/// came from the same visual line and must be placed at one shared final `y` with a single
/// cursor advance, not one advance per run (that was the actual bug: naively calling
/// `place_text_run` per run advanced the cursor once per span instead of once per line).
fn place_rich_inline_content(
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    content: &[md2pdf_ast::InlineNode],
    font_system: &mut FontSystem,
) {
    let shaped = shape_rich_paragraph(font_system, content, cursor.content_width_pt - indent_pt);
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
            let mut element = shaped_run.element;
            if let PositionedElement::TextRun { x, y, .. } = &mut element {
                *x += margin_pt + indent_pt;
                *y = placed_y;
            }
            cursor.current.push(element);
        }
        cursor.y = placed_y + line_height;
    }
}

fn render_block(
    block: &BlockNode,
    cursor: &mut Cursor,
    margin_pt: f32,
    indent_pt: f32,
    font_system: &mut FontSystem,
    images: &ImageTable,
    diagrams: &DiagramTable,
) {
    match block {
        BlockNode::Heading { content, .. } => {
            let heading_size = content.first().map(|c| c.style.size).unwrap_or(12.0);
            let heading_h = estimate_line_height(heading_size);
            if cursor.remaining_height() < heading_h && !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }
            place_inline_content(cursor, margin_pt, indent_pt, content, font_system);
        }
        BlockNode::Paragraph { content } => {
            place_inline_content(cursor, margin_pt, indent_pt, content, font_system);
        }
        BlockNode::Blockquote { content } => {
            let start_y = cursor.y;
            for child in content {
                render_block(child, cursor, margin_pt, indent_pt + BLOCKQUOTE_INDENT_PT, font_system, images, diagrams);
            }
            let end_y = cursor.y;
            cursor.current.push(PositionedElement::Path {
                points: vec![
                    PathCommand::MoveTo(margin_pt + indent_pt + 4.0, start_y),
                    PathCommand::LineTo(margin_pt + indent_pt + 4.0, end_y),
                ],
                fill: None,
                stroke: Some(StrokeStyle { color: [180, 180, 180], width: 2.0 }),
            });
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
        BlockNode::List { items, .. } => {
            for item in items {
                for child in item {
                    render_block(child, cursor, margin_pt, indent_pt + LIST_INDENT_PT, font_system, images, diagrams);
                }
            }
        }
        BlockNode::CodeBlock { tokens, .. } => {
            let start_y = cursor.y;
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
            place_rich_inline_content(cursor, margin_pt, indent_pt + 8.0, &combined, font_system);
            let end_y = cursor.y;
            cursor.current.push(PositionedElement::Path {
                points: vec![
                    PathCommand::MoveTo(margin_pt + indent_pt, start_y - 4.0),
                    PathCommand::LineTo(margin_pt + cursor.content_width_pt, start_y - 4.0),
                    PathCommand::LineTo(margin_pt + cursor.content_width_pt, end_y),
                    PathCommand::LineTo(margin_pt + indent_pt, end_y),
                    PathCommand::Close,
                ],
                fill: Some(CODE_BLOCK_BG),
                stroke: None,
            });
        }
        BlockNode::Table { headers, rows, .. } => {
            let widths = crate::table::column_widths(headers, rows, cursor.content_width_pt - indent_pt, font_system);
            let top_y = cursor.y;
            let row_height = 20.0;

            let mut col_x = margin_pt + indent_pt;
            for (header, width) in headers.iter().zip(&widths) {
                place_inline_content(cursor, margin_pt, col_x - margin_pt, std::slice::from_ref(header), font_system);
                col_x += width;
            }
            cursor.y = top_y + row_height;
            let header_bottom_y = cursor.y;

            for row in rows {
                let row_top_y = cursor.y;
                let mut col_x = margin_pt + indent_pt;
                for (cell, width) in row.iter().zip(&widths) {
                    place_inline_content(cursor, margin_pt, col_x - margin_pt, std::slice::from_ref(cell), font_system);
                    col_x += width;
                }
                cursor.y = row_top_y + row_height;
            }

            cursor.current.push(crate::table::grid_path(margin_pt + indent_pt, top_y, cursor.y, header_bottom_y, &widths));
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
                let (width, height) = if diagram.width > max_width {
                    (max_width, max_width * aspect)
                } else {
                    (diagram.width, diagram.height)
                };
                if cursor.remaining_height() < height && !cursor.current.is_empty() {
                    cursor.break_page(margin_pt);
                }
                cursor.current.push(PositionedElement::VectorGraphic { x: margin_pt + indent_pt, y: cursor.y, width, height, diagram_id: id.clone() });
                cursor.y += height;
            }
        }
    }
}

pub struct LayoutOutput {
    pub pages: Vec<PositionedPage>,
    pub images: ImageTable,
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
    for block in ast {
        render_block(block, &mut cursor, margin_pt, 0.0, font_system, &images, diagrams);
        cursor.y += LINE_SPACING_PT;
    }
    LayoutOutput { pages: cursor.finish(), images }
}
