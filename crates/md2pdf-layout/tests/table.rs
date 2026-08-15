use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).unwrap();
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn cell(text: &str) -> Vec<InlineNode> {
    vec![InlineNode { text: text.to_string(), style: TextStyle { bold: false, italic: false, size: 12.0, color: [0, 0, 0] }, link_target: None }]
}

#[test]
fn wider_column_content_gets_a_proportionally_wider_column() {
    let mut fs = test_font_system();
    let headers = vec![cell("A"), cell("A much longer header")];
    let rows = vec![vec![cell("x"), cell("y")]];
    let widths = md2pdf_layout::test_support::column_widths(&headers, &rows, 400.0, &mut fs);

    assert_eq!(widths.len(), 2);
    assert!(widths[1] > widths[0], "expected the longer-content column to be wider");
    let total: f32 = widths.iter().sum();
    assert!((total - 400.0).abs() < 0.5, "expected column widths to sum to the available width, got {total}");
}
