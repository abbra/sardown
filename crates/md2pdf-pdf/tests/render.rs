use md2pdf_layout::{PositionedElement, PositionedGlyph, PositionedPage};
use md2pdf_pdf::render_pdf;

fn test_font_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db
}

#[test]
fn renders_a_single_page_with_one_text_run_to_valid_pdf_bytes() {
    let db = test_font_db();
    let font_id = db.faces().next().expect("no faces in test font").id;

    // A single glyph run is enough to prove the krilla bridge works end-to-end;
    // exact glyph IDs come from Task 7's shape_paragraph in real use.
    let page = PositionedPage {
        page_number: 0,
        elements: vec![PositionedElement::TextRun {
            x: 72.0,
            y: 72.0,
            glyphs: vec![PositionedGlyph { glyph_id: 3, x: 0.0, y: 0.0, x_advance: 10.0 }],
            font_id,
            size: 12.0,
            color: [0, 0, 0],
        }],
    };

    let pdf_bytes = render_pdf(&[page], &db).expect("render_pdf failed");

    assert!(pdf_bytes.starts_with(b"%PDF-"), "output does not start with a PDF header");
    let doc = lopdf::Document::load_mem(&pdf_bytes).expect("krilla output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 1, "expected exactly one page");
}
