use cosmic_text::FontSystem;
use md2pdf_ast::{parse, BlockNode, InlineNode, TextStyle};
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{layout_impl, PageGeometry, PositionedElement};
use md2pdf_style::Stylesheet;

fn plain_inline(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn heading(level: u8, id: &str, text: &str) -> BlockNode {
    BlockNode::Heading { level, id: id.to_string(), content: vec![plain_inline(text)] }
}

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

/// Same shape as `book_with_headings`, but with enough filler text under Chapter One that Chapter
/// Two lands on a later physical page -- needed to test numbering resets, which only mean
/// anything when two headings end up on different pages.
fn book_with_headings_spanning_multiple_pages() -> Vec<BlockNode> {
    let filler = "Filler text to push the next heading onto a new physical page. ".repeat(200);
    parse(&format!("# Chapter One\n\n{filler}\n\n# Chapter Two\n\nBody after the break.\n"))
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
fn toc_entries_include_headings_nested_inside_blockquotes_lists_and_columns() {
    let ast = vec![
        heading(1, "top", "Top Level"),
        BlockNode::Blockquote { content: vec![heading(2, "in-quote", "In A Blockquote")] },
        BlockNode::List { ordered: false, start: None, items: vec![vec![heading(2, "in-list", "In A List Item")]] },
        BlockNode::Columns(vec![vec![heading(2, "in-column", "In A Column")]]),
    ];
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);

    let ids: Vec<&str> = output.toc_entries.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&"top"), "expected the top-level heading in the TOC, got {ids:?}");
    assert!(ids.contains(&"in-quote"), "expected the blockquote-nested heading in the TOC, got {ids:?}");
    assert!(ids.contains(&"in-list"), "expected the list-nested heading in the TOC, got {ids:?}");
    assert!(ids.contains(&"in-column"), "expected the column-nested heading in the TOC, got {ids:?}");
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
fn toc_entries_reflect_configured_numbering_resets() {
    let ast = book_with_headings_spanning_multiple_pages();
    let mut style = Stylesheet::default();
    style.toc.enabled = true;
    style.page.numbering.format = md2pdf_style::NumberingFormat::RomanLower;
    style.page.numbering.resets =
        vec![md2pdf_style::PageNumberingReset { at_heading: "chapter-two".to_string(), format: md2pdf_style::NumberingFormat::Arabic, start_at: 1 }];
    let mut fs = test_font_system();
    let mut output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);
    assert!(output.pages.len() >= 2, "expected the filler text to force Chapter Two onto a later physical page");
    md2pdf_layout::insert_table_of_contents(&mut output, &ast, &style, &letter_geometry(), &mut fs);

    let toc_texts: Vec<String> = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // Chapter One lands on physical page index 1 (page 0 is the TOC itself), so with the base
    // numbering's start_at = 1 that's roman_lower "ii", not "i".
    assert!(toc_texts.iter().any(|t| t == "ii"), "expected Chapter One's page number in roman_lower (before the reset): {toc_texts:?}");
    assert!(toc_texts.iter().any(|t| t == "1"), "expected Chapter Two's page number reset to arabic 1: {toc_texts:?}");
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
