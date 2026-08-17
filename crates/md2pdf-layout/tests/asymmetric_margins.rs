use md2pdf_ast::LinkTarget;
use md2pdf_layout::{apply_asymmetric_margins, PageGeometry, PathCommand, PositionedElement, PositionedPage, Rect};

const PT_PER_MM: f32 = 2.834_645_7;

fn geometry(inner_margin_mm: Option<f32>, outer_margin_mm: Option<f32>) -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4, inner_margin_mm, outer_margin_mm }
}

fn text_run_at(x: f32) -> PositionedElement {
    PositionedElement::TextRun { x, y: 100.0, glyphs: Vec::new(), text: String::new(), font_id: fontdb::ID::dummy(), size: 12.0, color: [0, 0, 0] }
}

fn x_of(element: &PositionedElement) -> f32 {
    match element {
        PositionedElement::TextRun { x, .. } => *x,
        _ => panic!("expected a TextRun"),
    }
}

#[test]
fn is_a_no_op_when_inner_and_outer_margins_are_not_configured() {
    let baseline_pt = 25.4 * PT_PER_MM;
    let mut pages = vec![
        PositionedPage { page_number: 0, elements: vec![text_run_at(baseline_pt)] },
        PositionedPage { page_number: 1, elements: vec![text_run_at(baseline_pt)] },
    ];
    apply_asymmetric_margins(&mut pages, &geometry(None, None));
    assert_eq!(x_of(&pages[0].elements[0]), baseline_pt);
    assert_eq!(x_of(&pages[1].elements[0]), baseline_pt);
}

#[test]
fn recto_pages_end_up_at_the_inner_margin_and_verso_pages_at_the_outer_margin() {
    let baseline_pt = 25.4 * PT_PER_MM;
    let inner_pt = 30.0 * PT_PER_MM;
    let outer_pt = 15.0 * PT_PER_MM;
    let mut pages = vec![
        PositionedPage { page_number: 0, elements: vec![text_run_at(baseline_pt)] }, // recto (1st physical page)
        PositionedPage { page_number: 1, elements: vec![text_run_at(baseline_pt)] }, // verso (2nd physical page)
        PositionedPage { page_number: 2, elements: vec![text_run_at(baseline_pt)] }, // recto (3rd physical page)
    ];
    apply_asymmetric_margins(&mut pages, &geometry(Some(30.0), Some(15.0)));
    assert!((x_of(&pages[0].elements[0]) - inner_pt).abs() < 0.01, "expected recto page 1 at the inner margin");
    assert!((x_of(&pages[1].elements[0]) - outer_pt).abs() < 0.01, "expected verso page 2 at the outer margin");
    assert!((x_of(&pages[2].elements[0]) - inner_pt).abs() < 0.01, "expected recto page 3 at the inner margin");
}

#[test]
fn shifts_every_element_kind_that_carries_an_x_coordinate() {
    let baseline_pt = 25.4 * PT_PER_MM;
    let inner_pt = 30.0 * PT_PER_MM;
    let mut pages = vec![PositionedPage {
        page_number: 0,
        elements: vec![
            PositionedElement::Path {
                points: vec![PathCommand::MoveTo(baseline_pt, 10.0), PathCommand::LineTo(baseline_pt + 5.0, 10.0)],
                fill: None,
                stroke: None,
            },
            PositionedElement::LinkAnnotation {
                rect: Rect { x: baseline_pt, y: 0.0, width: 10.0, height: 10.0 },
                destination: LinkTarget::InternalAnchor("x".to_string()),
            },
            PositionedElement::RasterImage { x: baseline_pt, y: 0.0, width: 10.0, height: 10.0, image_id: "img".to_string() },
            PositionedElement::VectorGraphic { x: baseline_pt, y: 0.0, width: 10.0, height: 10.0, diagram_id: "d".to_string() },
        ],
    }];
    apply_asymmetric_margins(&mut pages, &geometry(Some(30.0), Some(15.0)));
    let shift = inner_pt - baseline_pt;

    for element in &pages[0].elements {
        match element {
            PositionedElement::Path { points, .. } => {
                let PathCommand::MoveTo(x, _) = points[0] else { panic!("expected MoveTo") };
                assert!((x - (baseline_pt + shift)).abs() < 0.01);
            }
            PositionedElement::LinkAnnotation { rect, .. } => assert!((rect.x - (baseline_pt + shift)).abs() < 0.01),
            PositionedElement::RasterImage { x, .. } => assert!((x - (baseline_pt + shift)).abs() < 0.01),
            PositionedElement::VectorGraphic { x, .. } => assert!((x - (baseline_pt + shift)).abs() < 0.01),
            PositionedElement::TextRun { .. } => panic!("unexpected TextRun"),
        }
    }
}
