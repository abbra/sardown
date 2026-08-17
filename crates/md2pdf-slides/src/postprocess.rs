use md2pdf_layout::{shift_element, PathCommand, PositionedElement, PositionedPage};
use md2pdf_style::Color;

/// Shifts every element on `page` down by `(page_height_pt - content_height_pt) / 2 - top_y`,
/// where `content_height_pt` is the page's own vertical extent (bottom-most edge minus top-most
/// edge) -- centers the page's content vertically instead of leaving it pinned to the top margin,
/// the same "layout once top-anchored, then shift" approach `apply_asymmetric_margins` uses for
/// horizontal shifts. A no-op if the page has no measurable content, or if its content already
/// fills at least half the page (centering would otherwise push it upward, off the page).
pub fn center_vertically(page: &mut PositionedPage, page_height_pt: f32) {
    let Some((top_y, bottom_y)) = vertical_extent(page) else { return };
    let content_height_pt = bottom_y - top_y;
    let shift_pt = (page_height_pt - content_height_pt) / 2.0 - top_y;
    if shift_pt <= 0.0 {
        return;
    }
    for element in &mut page.elements {
        shift_element(element, 0.0, shift_pt);
    }
}

/// Prepends one full-page filled rectangle to `page.elements` -- drawn first so every other
/// element on the page layers on top of it, mirroring how a code block's own background is
/// inserted before its text.
pub fn fill_background(page: &mut PositionedPage, color: Color, page_width_pt: f32, page_height_pt: f32) {
    let rect = PositionedElement::Path {
        points: vec![
            PathCommand::MoveTo(0.0, 0.0),
            PathCommand::LineTo(page_width_pt, 0.0),
            PathCommand::LineTo(page_width_pt, page_height_pt),
            PathCommand::LineTo(0.0, page_height_pt),
            PathCommand::Close,
        ],
        fill: Some(color.0),
        stroke: None,
    };
    page.elements.insert(0, rect);
}

fn vertical_extent(page: &PositionedPage) -> Option<(f32, f32)> {
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    let mut found = false;
    for element in &page.elements {
        let bounds = match element {
            PositionedElement::TextRun { y, size, .. } => Some((*y - *size, *y + *size * 0.3)),
            PositionedElement::Path { points, .. } => path_extent(points),
            PositionedElement::VectorGraphic { y, height, .. } => Some((*y, *y + *height)),
            PositionedElement::RasterImage { y, height, .. } => Some((*y, *y + *height)),
            PositionedElement::LinkAnnotation { .. } => None,
        };
        if let Some((top, bottom)) = bounds {
            min_y = min_y.min(top);
            max_y = max_y.max(bottom);
            found = true;
        }
    }
    found.then_some((min_y, max_y))
}

fn path_extent(points: &[PathCommand]) -> Option<(f32, f32)> {
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    let mut found = false;
    for point in points {
        let y = match point {
            PathCommand::MoveTo(_, y) | PathCommand::LineTo(_, y) => Some(*y),
            PathCommand::CubicTo(_, _, _, _, _, y3) => Some(*y3),
            PathCommand::Close => None,
        };
        if let Some(y) = y {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            found = true;
        }
    }
    found.then_some((min_y, max_y))
}
