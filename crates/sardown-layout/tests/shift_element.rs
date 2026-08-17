use sardown_ast::LinkTarget;
use sardown_layout::{shift_element, PathCommand, PositionedElement, Rect};

fn test_font_id() -> fontdb::ID {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).unwrap();
    let id = db.faces().next().unwrap().id;
    id
}

#[test]
fn shift_element_moves_a_text_run_by_dx_and_dy() {
    let mut element =
        PositionedElement::TextRun { x: 10.0, y: 20.0, glyphs: Vec::new(), text: String::new(), font_id: test_font_id(), size: 12.0, color: [0, 0, 0] };
    shift_element(&mut element, 5.0, -3.0);
    let PositionedElement::TextRun { x, y, .. } = element else { unreachable!() };
    assert_eq!(x, 15.0);
    assert_eq!(y, 17.0);
}

#[test]
fn shift_element_moves_every_point_of_a_path() {
    let mut element = PositionedElement::Path {
        points: vec![PathCommand::MoveTo(0.0, 0.0), PathCommand::LineTo(10.0, 10.0), PathCommand::CubicTo(1.0, 1.0, 2.0, 2.0, 3.0, 3.0), PathCommand::Close],
        fill: None,
        stroke: None,
    };
    shift_element(&mut element, 2.0, 4.0);
    let PositionedElement::Path { points, .. } = element else { unreachable!() };
    assert_eq!(points[0], PathCommand::MoveTo(2.0, 4.0));
    assert_eq!(points[1], PathCommand::LineTo(12.0, 14.0));
    assert_eq!(points[2], PathCommand::CubicTo(3.0, 5.0, 4.0, 6.0, 5.0, 7.0));
    assert_eq!(points[3], PathCommand::Close);
}

#[test]
fn shift_element_moves_a_link_annotations_rect() {
    let mut element =
        PositionedElement::LinkAnnotation { rect: Rect { x: 1.0, y: 2.0, width: 3.0, height: 4.0 }, destination: LinkTarget::InternalAnchor("x".to_string()) };
    shift_element(&mut element, 1.0, 1.0);
    let PositionedElement::LinkAnnotation { rect, .. } = element else { unreachable!() };
    assert_eq!((rect.x, rect.y), (2.0, 3.0));
}
