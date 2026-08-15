use crate::{shape_paragraph, PathCommand, PositionedElement, StrokeStyle};
use cosmic_text::FontSystem;
use md2pdf_ast::InlineNode;

/// Pass 1: measure each column's minimum required width (longest single line, unwrapped).
/// Pass 2: distribute available width proportionally to those minimums, floored at a
/// readable minimum so no column collapses to zero.
pub fn column_widths(
    headers: &[Vec<InlineNode>],
    rows: &[Vec<Vec<InlineNode>>],
    available_width_pt: f32,
    font_system: &mut FontSystem,
) -> Vec<f32> {
    let column_count = headers.len();
    let mut minimums = vec![0.0f32; column_count];

    for (col, header) in headers.iter().enumerate() {
        minimums[col] = minimums[col].max(measure_unwrapped_width(font_system, header));
    }
    for row in rows {
        for (col, cell) in row.iter().enumerate() {
            if col < column_count {
                minimums[col] = minimums[col].max(measure_unwrapped_width(font_system, cell));
            }
        }
    }

    const MIN_COLUMN_WIDTH_PT: f32 = 40.0;
    let total_minimum: f32 = minimums.iter().sum();
    if total_minimum <= 0.0 {
        return vec![available_width_pt / column_count as f32; column_count];
    }
    minimums
        .iter()
        .map(|m| (m / total_minimum * available_width_pt).max(MIN_COLUMN_WIDTH_PT))
        .collect()
}

fn measure_unwrapped_width(font_system: &mut FontSystem, cell: &[InlineNode]) -> f32 {
    let elements = shape_paragraph(font_system, cell, f32::MAX);
    elements
        .into_iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { glyphs, .. } => Some(glyphs.iter().map(|g| g.x_advance).sum::<f32>()),
            _ => None,
        })
        .fold(0.0, f32::max)
        + 12.0 // cell padding
}

/// Grid line Path for a table occupying [x, x + total_width] x [top_y, bottom_y], with one
/// vertical line per column boundary and, if this page segment includes the header row, one
/// horizontal line under it. `header_bottom_y` is `None` for a page a table continues onto
/// after a row-level page break, since the header is not repeated there.
pub(crate) fn grid_path(x: f32, top_y: f32, bottom_y: f32, header_bottom_y: Option<f32>, widths: &[f32]) -> PositionedElement {
    let mut points = vec![PathCommand::MoveTo(x, top_y)];
    let mut cursor_x = x;
    for width in widths {
        cursor_x += width;
        points.push(PathCommand::MoveTo(cursor_x, top_y));
        points.push(PathCommand::LineTo(cursor_x, bottom_y));
    }
    if let Some(header_bottom_y) = header_bottom_y {
        points.push(PathCommand::MoveTo(x, header_bottom_y));
        points.push(PathCommand::LineTo(cursor_x, header_bottom_y));
    }
    PositionedElement::Path { points, fill: None, stroke: Some(StrokeStyle { color: [180, 180, 180], width: 0.75 }) }
}
