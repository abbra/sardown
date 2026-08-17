use cosmic_text::FontSystem;
use sardown_ast::{parse, BlockNode, InlineNode, TextStyle};
use sardown_enrich::DiagramTable;
use sardown_layout::{layout, PageGeometry};

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

fn plain_inline(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

#[test]
fn page_context_tracks_the_most_recent_h1_and_h2() {
    let ast = parse("# Chapter One\n\nBody\n\n## Section A\n\nMore body\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert_eq!(output.page_contexts.len(), 1);
    assert_eq!(output.page_contexts[0].current_h1.as_deref(), Some("Chapter One"));
    assert_eq!(output.page_contexts[0].current_h2.as_deref(), Some("Section A"));
}

#[test]
fn a_new_h1_clears_the_previous_h2() {
    let ast = parse("# Chapter One\n\n## Section A\n\n# Chapter Two\n\nBody\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert_eq!(output.page_contexts[0].current_h1.as_deref(), Some("Chapter Two"));
    assert_eq!(output.page_contexts[0].current_h2, None);
}

#[test]
fn a_page_starting_with_an_h1_is_flagged_a_chapter_opener() {
    let ast = parse("# Chapter One\n\nBody\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert!(output.page_contexts[0].is_chapter_opener);
}

#[test]
fn a_page_not_starting_with_a_heading_is_not_a_chapter_opener() {
    let ast = parse("Intro paragraph.\n\n# Chapter One\n\nBody\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert!(!output.page_contexts[0].is_chapter_opener);
}

#[test]
fn a_page_starting_with_an_h2_is_not_flagged_a_chapter_opener() {
    let ast = parse("## Section A\n\nBody\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert!(!output.page_contexts[0].is_chapter_opener);
}

#[test]
fn page_contexts_has_exactly_one_entry_per_page() {
    let md: String = (0..60).map(|i| format!("# Heading {i}\n\n")).collect();
    let ast = parse(&md);
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert!(output.pages.len() >= 2, "expected the 60 headings to overflow onto a second page");
    assert_eq!(output.page_contexts.len(), output.pages.len());
}

#[test]
fn a_forced_page_break_starts_a_fresh_chapter_context_on_the_next_page() {
    let ast = vec![
        BlockNode::Heading { level: 1, id: "ch1".to_string(), content: vec![plain_inline("Chapter One")] },
        BlockNode::Paragraph { content: vec![plain_inline("Body of chapter one.")] },
        BlockNode::PageBreak,
        BlockNode::Heading { level: 1, id: "ch2".to_string(), content: vec![plain_inline("Chapter Two")] },
        BlockNode::Paragraph { content: vec![plain_inline("Body of chapter two.")] },
    ];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    assert_eq!(output.page_contexts.len(), 2);
    assert_eq!(output.page_contexts[0].current_h1.as_deref(), Some("Chapter One"));
    assert!(output.page_contexts[0].is_chapter_opener);
    assert_eq!(output.page_contexts[1].current_h1.as_deref(), Some("Chapter Two"));
    assert!(output.page_contexts[1].is_chapter_opener);
}
