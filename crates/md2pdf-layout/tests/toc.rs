use cosmic_text::FontSystem;
use md2pdf_ast::{parse, BlockNode};
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{layout_impl, PageGeometry, PositionedElement};
use md2pdf_style::Stylesheet;

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn letter_geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4, ..Default::default() }
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn book_with_headings() -> Vec<BlockNode> {
    parse("# Chapter One\n\nBody one.\n\n## Section A\n\nBody two.\n\n# Chapter Two\n\nBody three.\n")
}

#[test]
fn disabled_toc_leaves_layout_output_unchanged() {
    let ast = book_with_headings();
    let mut style = Stylesheet::default();
    style.toc.enabled = false;
    let mut fs = test_font_system();
    let without_call = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let mut fs2 = test_font_system();
    let mut with_toc_fn_called = layout_impl(&ast, &letter_geometry(), &mut fs2, &fixtures_dir(), &DiagramTable::new(), &style);
    md2pdf_layout::insert_table_of_contents(&mut with_toc_fn_called, &ast, &style, &letter_geometry(), &mut fs2);

    assert_eq!(without_call.pages.len(), with_toc_fn_called.pages.len(), "disabled TOC must add no pages");
    assert!(with_toc_fn_called.toc_entries.is_empty());
}

#[test]
fn enabled_toc_prepends_pages_and_shifts_existing_page_numbers_and_anchors() {
    let ast = book_with_headings();
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    style.toc.depth = 2;
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    let pages_before_toc = output.pages.len();
    let chapter_two_page_before = output.anchors.values().map(|a| a.page).max().unwrap();

    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);

    assert!(output.pages.len() > pages_before_toc, "expected at least one TOC page to be prepended");
    let toc_page_count = output.pages.len() - pages_before_toc;
    assert_eq!(output.pages[0].page_number, 0, "expected the first TOC page to be page 0");
    let chapter_two_page_after = output.anchors.values().map(|a| a.page).max().unwrap();
    assert_eq!(chapter_two_page_after, chapter_two_page_before + toc_page_count, "expected every anchor to shift by exactly the TOC's own page count");
    assert_eq!(output.toc_entries.len(), 3, "expected Chapter One, Section A, Chapter Two (all at or above depth 2)");
}

#[test]
fn toc_depth_one_excludes_h2_headings() {
    let ast = book_with_headings();
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    style.toc.depth = 1;
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);
    assert_eq!(output.toc_entries.len(), 2, "expected only the two H1s (Chapter One, Chapter Two)");
}

#[test]
fn a_document_with_no_matching_headings_gets_no_toc_page() {
    let ast = parse("Just a paragraph, no headings.\n");
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    let pages_before = output.pages.len();
    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);
    assert_eq!(output.pages.len(), pages_before, "expected no TOC page when there are no matching headings");
}

#[test]
fn toc_entries_link_to_their_target_heading_position() {
    let ast = book_with_headings();
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);

    let chapter_one_anchor = *output.anchors.get("chapter-one").expect("expected a chapter-one anchor");
    let link_targets_toc_page_zero = output.pages[0].elements.iter().any(|e| {
        matches!(
            e,
            PositionedElement::LinkAnnotation { destination: md2pdf_ast::LinkTarget::InternalAnchor(id), .. } if id == "chapter-one"
        )
    });
    assert!(link_targets_toc_page_zero, "expected a LinkAnnotation on the TOC page targeting chapter-one");
    assert!(chapter_one_anchor.page < output.pages.len());
}

#[test]
fn layout_with_header_footer_includes_the_toc_when_enabled() {
    let ast = book_with_headings();
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    let mut fs = test_font_system();
    let output = md2pdf_layout::layout_with_header_footer(&ast, &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    assert!(!output.toc_entries.is_empty(), "expected layout_with_header_footer to run TOC generation when enabled");
    let has_toc_title = output.pages[0].elements.iter().any(|e| {
        matches!(
            e,
            PositionedElement::TextRun { text, .. } if text.contains("Table of Contents")
        )
    });
    assert!(has_toc_title, "expected the TOC title on the first page");
}
