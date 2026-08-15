use crate::{shape_paragraph, PageGeometry, PathCommand, PositionedElement, PositionedPage, StrokeStyle};
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;

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

fn render_block(block: &BlockNode, cursor: &mut Cursor, margin_pt: f32, indent_pt: f32, font_system: &mut FontSystem) {
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
                render_block(child, cursor, margin_pt, indent_pt + BLOCKQUOTE_INDENT_PT, font_system);
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
                    render_block(child, cursor, margin_pt, indent_pt + LIST_INDENT_PT, font_system);
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
            // Split into one shape_paragraph call per HighlightedToken so each keeps its own
            // color; a single combined call would flatten all tokens to the first run's color
            // (shape_paragraph's Phase 1 limitation, see its doc comment).
            for run in &combined {
                place_inline_content(cursor, margin_pt, indent_pt + 8.0, std::slice::from_ref(run), font_system);
            }
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
        BlockNode::Table { .. } => {} // Task 4
        BlockNode::Image { .. } => {} // Task 5
        BlockNode::MermaidDiagram { .. } => {} // Phase 3
    }
}

pub fn layout(ast: &[BlockNode], geometry: &PageGeometry, font_system: &mut FontSystem) -> Vec<PositionedPage> {
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let mut cursor = Cursor::new(geometry);
    for block in ast {
        render_block(block, &mut cursor, margin_pt, 0.0, font_system);
        cursor.y += LINE_SPACING_PT;
    }
    cursor.finish()
}
