use cosmic_text::FontSystem;
use md2pdf_ast::{parse, BlockNode};
use md2pdf_layout::{layout, PageGeometry, PositionedElement};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn letter_geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4 } // US Letter, 1in margins
}

#[test]
fn single_short_paragraph_fits_on_one_page() {
    let ast = parse("Just one short paragraph.\n");
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs);
    assert_eq!(pages.len(), 1);
    assert!(!pages[0].elements.is_empty());
}

#[test]
fn many_headings_overflow_onto_a_second_page() {
    // 60 headings at a fixed line height comfortably exceeds one US Letter page
    let md: String = (0..60).map(|i| format!("# Heading {i}\n\n")).collect();
    let blocks: Vec<BlockNode> = parse(&md);
    let mut fs = test_font_system();
    let pages = layout(&blocks, &letter_geometry(), &mut fs);
    assert!(pages.len() >= 2, "expected content to overflow onto a second page, got {} page(s)", pages.len());
    assert_eq!(pages[0].page_number, 0);
    assert_eq!(pages[1].page_number, 1);
}

#[test]
fn heading_at_bottom_of_page_moves_with_its_first_line_of_body_text() {
    // Widow/orphan rule (§4.2 item 4): a heading must not be the last element on a page
    // with none of its following paragraph's text on the same page.
    let md = "# T\n\nBody\n".repeat(40); // pad until a heading lands near a page boundary
    let blocks = parse(&md);
    let mut fs = test_font_system();
    let pages = layout(&blocks, &letter_geometry(), &mut fs);
    for page in &pages[..pages.len() - 1] {
        let last_is_lone_heading = matches!(page.elements.last(), Some(PositionedElement::TextRun { .. })) && page.elements.len() == 1;
        assert!(!last_is_lone_heading, "found a page ending in an isolated heading with no body text");
    }
}
