use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{AnchorPosition, AnchorTable, ImageTable, PositionedElement, PositionedGlyph, PositionedPage, TocEntry};
use md2pdf_pdf::render_pdf;

fn test_font_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db
}

fn one_page_with_text(font_id: fontdb::ID) -> PositionedPage {
    PositionedPage {
        page_number: 0,
        elements: vec![PositionedElement::TextRun {
            x: 72.0,
            y: 72.0,
            glyphs: vec![PositionedGlyph { glyph_id: 3, x: 0.0, y: 0.0, x_advance: 10.0, cluster: 0..1 }],
            text: "x".to_string(),
            font_id,
            size: 12.0,
            color: [0, 0, 0],
        }],
    }
}

#[test]
fn toc_entries_produce_a_non_empty_pdf_outline() {
    let db = test_font_db();
    let font_id = db.faces().next().unwrap().id;
    let page = one_page_with_text(font_id);
    let mut anchors = AnchorTable::new();
    anchors.insert("chapter-one".to_string(), AnchorPosition { page: 0, x: 72.0, y: 72.0 });
    let toc_entries = vec![TocEntry { level: 1, id: "chapter-one".to_string(), text: "Chapter One".to_string() }];

    let pdf_bytes = render_pdf(&[page], &db, &ImageTable::new(), &DiagramTable::new(), &anchors, 612.0, 792.0, &toc_entries).unwrap();
    let doc = lopdf::Document::load_mem(&pdf_bytes).unwrap();
    let has_outlines = doc.catalog().ok().and_then(|cat| cat.get(b"Outlines").ok()).is_some();
    assert!(has_outlines, "expected a non-empty /Outlines entry in the document catalog");
}

#[test]
fn no_toc_entries_means_no_pdf_outline() {
    let db = test_font_db();
    let font_id = db.faces().next().unwrap().id;
    let page = one_page_with_text(font_id);
    let pdf_bytes = render_pdf(&[page], &db, &ImageTable::new(), &DiagramTable::new(), &AnchorTable::new(), 612.0, 792.0, &[]).unwrap();
    let doc = lopdf::Document::load_mem(&pdf_bytes).unwrap();
    let has_outlines = doc.catalog().ok().and_then(|cat| cat.get(b"Outlines").ok()).is_some();
    assert!(!has_outlines, "expected no /Outlines entry when there are no TOC entries");
}
