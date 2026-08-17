use md2pdf_layout::{PathCommand, PositionedElement, PositionedPage};
use md2pdf_slides::{center_vertically, fill_background};
use md2pdf_style::Color;

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
