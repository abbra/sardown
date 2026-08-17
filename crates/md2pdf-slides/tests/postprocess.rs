use md2pdf_layout::{PathCommand, PositionedElement, PositionedPage};
use md2pdf_slides::{center_vertically, draw_background_diagram, draw_background_image, fill_background};
use md2pdf_style::{Color, ImageCorner};

fn test_font_id() -> fontdb::ID {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).unwrap();
    let id = db.faces().next().unwrap().id;
    id
}

fn text_run_at(y: f32, size: f32) -> PositionedElement {
    PositionedElement::TextRun { x: 0.0, y, glyphs: Vec::new(), text: String::new(), font_id: test_font_id(), size, color: [0, 0, 0] }
}

#[test]
fn centering_shifts_content_so_its_midpoint_matches_the_pages_midpoint() {
    // A single 12pt text run whose baseline sits at y = 20 (so its visual span is roughly
    // [20 - 12, 20 + 12*0.3] = [8, 23.6]) on a 200pt-tall page: after centering, the midpoint of
    // that span should land at the page's own midpoint (100).
    let mut page = PositionedPage { page_number: 0, elements: vec![text_run_at(20.0, 12.0)] };
    center_vertically(&mut page, 200.0);
    let PositionedElement::TextRun { y, size, .. } = &page.elements[0] else { unreachable!() };
    let (top, bottom) = (*y - *size, *y + *size * 0.3);
    let midpoint = (top + bottom) / 2.0;
    assert!((midpoint - 100.0).abs() < 1.0, "expected the shifted content's midpoint near 100.0, got {midpoint}");
}

#[test]
fn centering_accounts_for_a_cubic_curves_control_points_not_just_its_endpoint() {
    // A cubic curve starting and ending at y = 100 but bulging up to y = 10/20 via its control
    // points: the curve's own visual extent reaches those control points, not just its endpoint.
    let path = PositionedElement::Path {
        points: vec![PathCommand::MoveTo(0.0, 100.0), PathCommand::CubicTo(10.0, 10.0, 20.0, 20.0, 30.0, 100.0)],
        fill: None,
        stroke: Some(md2pdf_layout::StrokeStyle { color: [0, 0, 0], width: 1.0 }),
    };
    let mut page = PositionedPage { page_number: 0, elements: vec![path] };
    center_vertically(&mut page, 200.0);
    let PositionedElement::Path { points, .. } = &page.elements[0] else { unreachable!() };
    let PathCommand::MoveTo(_, shifted_y) = points[0] else { unreachable!() };
    // Ignoring the control points would see a zero-height span at y=100 on a 200pt page and
    // treat it as already filling more than half (a same-as-page-height span), leaving it
    // unshifted. Accounting for the control points' true [10, 100] extent yields a real shift.
    assert!((shifted_y - 100.0).abs() > 1.0, "expected centering to actually move the curve, got shifted_y={shifted_y}");
}

#[test]
fn centering_a_page_with_no_elements_does_nothing() {
    let mut page = PositionedPage { page_number: 0, elements: Vec::new() };
    center_vertically(&mut page, 200.0);
    assert!(page.elements.is_empty());
}

#[test]
fn centering_content_that_already_fills_more_than_half_the_page_leaves_it_untouched() {
    let elements = vec![text_run_at(20.0, 12.0), text_run_at(190.0, 12.0)];
    let mut page = PositionedPage { page_number: 0, elements: elements.clone() };
    center_vertically(&mut page, 200.0);
    let PositionedElement::TextRun { y: y0, .. } = &page.elements[0] else { unreachable!() };
    let PositionedElement::TextRun { y: original_y0, .. } = &elements[0] else { unreachable!() };
    assert_eq!(y0, original_y0, "content spanning most of the page should not be pushed further");
}

#[test]
fn fill_background_prepends_a_full_page_filled_rectangle() {
    let mut page = PositionedPage { page_number: 0, elements: vec![text_run_at(20.0, 12.0)] };
    fill_background(&mut page, Color([27, 13, 51]), 300.0, 200.0);
    assert_eq!(page.elements.len(), 2);
    let PositionedElement::Path { points, fill, stroke } = &page.elements[0] else { panic!("expected a Path as the first element") };
    assert_eq!(*fill, Some([27, 13, 51]));
    assert!(stroke.is_none());
    assert!(points.contains(&PathCommand::MoveTo(0.0, 0.0)));
    assert!(points.contains(&PathCommand::LineTo(300.0, 200.0)));
    assert!(matches!(page.elements[1], PositionedElement::TextRun { .. }), "the original text must still be present, drawn after the background");
}

fn image_position(corner: ImageCorner) -> (f32, f32) {
    let mut page = PositionedPage { page_number: 0, elements: Vec::new() };
    draw_background_image(&mut page, "logo.png", corner, 60.0, 40.0, 10.0, 300.0, 200.0);
    match &page.elements[0] {
        PositionedElement::RasterImage { x, y, .. } => (*x, *y),
        other => panic!("expected RasterImage, got {other:?}"),
    }
}

#[test]
fn draw_background_image_positions_each_corner_correctly() {
    // 300x200pt page, a 60x40pt image, 10pt margin from both nearest edges.
    assert_eq!(image_position(ImageCorner::TopLeft), (10.0, 10.0));
    assert_eq!(image_position(ImageCorner::TopRight), (300.0 - 10.0 - 60.0, 10.0));
    assert_eq!(image_position(ImageCorner::BottomLeft), (10.0, 200.0 - 10.0 - 40.0));
    assert_eq!(image_position(ImageCorner::BottomRight), (300.0 - 10.0 - 60.0, 200.0 - 10.0 - 40.0));
}

#[test]
fn draw_background_image_inserts_before_existing_content() {
    let mut page = PositionedPage { page_number: 0, elements: vec![text_run_at(20.0, 12.0)] };
    draw_background_image(&mut page, "logo.png", ImageCorner::BottomRight, 60.0, 40.0, 10.0, 300.0, 200.0);
    assert_eq!(page.elements.len(), 2);
    assert!(matches!(page.elements[0], PositionedElement::RasterImage { .. }));
    assert!(matches!(page.elements[1], PositionedElement::TextRun { .. }));
}

#[test]
fn draw_background_diagram_positions_using_the_same_corner_math_as_the_raster_version() {
    let mut page = PositionedPage { page_number: 0, elements: Vec::new() };
    draw_background_diagram(&mut page, "logo.svg", ImageCorner::TopRight, 60.0, 40.0, 10.0, 300.0, 200.0);
    match &page.elements[0] {
        PositionedElement::VectorGraphic { x, y, width, height, diagram_id } => {
            assert_eq!((*x, *y), (300.0 - 10.0 - 60.0, 10.0));
            assert_eq!((*width, *height), (60.0, 40.0));
            assert_eq!(diagram_id, "logo.svg");
        }
        other => panic!("expected VectorGraphic, got {other:?}"),
    }
}

#[test]
fn draw_background_diagram_inserts_before_existing_content() {
    let mut page = PositionedPage { page_number: 0, elements: vec![text_run_at(20.0, 12.0)] };
    draw_background_diagram(&mut page, "logo.svg", ImageCorner::BottomRight, 60.0, 40.0, 10.0, 300.0, 200.0);
    assert_eq!(page.elements.len(), 2);
    assert!(matches!(page.elements[0], PositionedElement::VectorGraphic { .. }));
    assert!(matches!(page.elements[1], PositionedElement::TextRun { .. }));
}

#[test]
fn a_background_image_drawn_before_fill_background_ends_up_between_the_fill_and_content() {
    // render_slide_deck calls draw_background_image *before* fill_background -- each inserts at
    // index 0, so calling them in that order produces the correct final paint order: fill
    // (bottom), then image, then whatever content was already on the page.
    let mut page = PositionedPage { page_number: 0, elements: vec![text_run_at(20.0, 12.0)] };
    draw_background_image(&mut page, "logo.png", ImageCorner::BottomRight, 60.0, 40.0, 10.0, 300.0, 200.0);
    fill_background(&mut page, Color([27, 13, 51]), 300.0, 200.0);
    assert!(matches!(page.elements[0], PositionedElement::Path { .. }), "background fill paints first (bottommost)");
    assert!(matches!(page.elements[1], PositionedElement::RasterImage { .. }), "then the background image");
    assert!(matches!(page.elements[2], PositionedElement::TextRun { .. }), "then the slide's own content");
}
