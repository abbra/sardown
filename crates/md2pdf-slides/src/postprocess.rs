use md2pdf_layout::{shift_element, PathCommand, PositionedElement, PositionedPage};
use md2pdf_style::{Color, ImageCorner};

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

/// Prepends a positioned raster image (already decoded and present in the deck's own `ImageTable`
/// under `image_id`) anchored to one corner of the page, `margin_pt` from both nearest edges.
/// Inserted the same way `fill_background` is (index 0) -- call this *before* `fill_background`
/// so the final paint order comes out fill (bottom), then image, then the slide's own content:
/// each `insert(0, ..)` pushes the previous first element to index 1, so inserting in that order
/// naturally produces it.
#[allow(clippy::too_many_arguments)]
pub fn draw_background_image(
    page: &mut PositionedPage,
    image_id: &str,
    corner: ImageCorner,
    width_pt: f32,
    height_pt: f32,
    margin_pt: f32,
    page_width_pt: f32,
    page_height_pt: f32,
) {
    let (x, y) = match corner {
        ImageCorner::TopLeft => (margin_pt, margin_pt),
        ImageCorner::TopRight => (page_width_pt - margin_pt - width_pt, margin_pt),
        ImageCorner::BottomLeft => (margin_pt, page_height_pt - margin_pt - height_pt),
        ImageCorner::BottomRight => (page_width_pt - margin_pt - width_pt, page_height_pt - margin_pt - height_pt),
    };
    page.elements.insert(0, PositionedElement::RasterImage { x, y, width: width_pt, height: height_pt, image_id: image_id.to_string() });
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
