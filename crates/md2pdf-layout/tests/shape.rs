use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};
use md2pdf_layout::{shape_paragraph, PositionedElement};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn plain_run(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, size: 12.0, color: [0, 0, 0] },
        link_target: None,
    }
}

#[test]
fn shapes_a_short_paragraph_into_one_text_run_with_glyphs() {
    let mut font_system = test_font_system();
    let content = vec![plain_run("Hello, world!")];
    let elements = shape_paragraph(&mut font_system, &content, 400.0);

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        PositionedElement::TextRun { glyphs, size, .. } => {
            assert!(!glyphs.is_empty(), "expected shaped glyphs, got none");
            assert_eq!(*size, 12.0);
        }
        other => panic!("expected TextRun, got a different variant: {other:?}"),
    }
}

#[test]
fn wraps_long_paragraph_into_multiple_lines() {
    let mut font_system = test_font_system();
    let long_text = "word ".repeat(80);
    let content = vec![plain_run(long_text.trim())];
    let elements = shape_paragraph(&mut font_system, &content, 200.0);
    assert!(elements.len() > 1, "expected line wrapping to produce multiple TextRuns");
}
