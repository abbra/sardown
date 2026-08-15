use crate::{shape_paragraph, PageGeometry, PositionedElement, PositionedPage};
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;

const PT_PER_MM: f32 = 2.834645669;
const LINE_SPACING_PT: f32 = 4.0; // gap after each block

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

pub fn layout(ast: &[BlockNode], geometry: &PageGeometry, font_system: &mut FontSystem) -> Vec<PositionedPage> {
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let mut cursor = Cursor::new(geometry);

    let mut index = 0;
    while index < ast.len() {
        let block = &ast[index];
        let content = match block {
            BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => content,
            _ => {
                index += 1;
                continue; // remaining variants handled in Task 3
            }
        };

        // Widow/orphan: if this is a Heading and it's the last block, or the next block isn't
        // a Paragraph, there's no "body" to keep it with — place normally.
        let heading_size = if let BlockNode::Heading { .. } = block {
            content.first().map(|c| c.style.size)
        } else {
            None
        };
        if let Some(size) = heading_size {
            let heading_h = estimate_line_height(size);
            let next_body_h = ast
                .get(index + 1)
                .filter(|b| matches!(b, BlockNode::Paragraph { .. }))
                .map(|_| estimate_line_height(12.0))
                .unwrap_or(0.0);
            if cursor.remaining_height() < heading_h + next_body_h && !cursor.current.is_empty() {
                cursor.break_page(margin_pt);
            }
        }

        let elements = shape_paragraph(font_system, content, cursor.content_width_pt);
        for mut element in elements {
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
        cursor.y += LINE_SPACING_PT;
        index += 1;
    }

    cursor.finish()
}
