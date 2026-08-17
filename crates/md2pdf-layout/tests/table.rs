use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).unwrap();
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn cell(text: &str) -> Vec<InlineNode> {
    vec![InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }]
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

#[test]
fn narrow_column_width_is_unaffected_by_how_long_a_neighboring_columns_content_is() {
    // Regression test: column width used to be purely proportional to each column's longest
    // unwrapped line, so a short column's share shrank whenever a neighboring column's content
    // grew longer, even with plenty of total width to go around -- forcing the short column's
    // own words to wrap mid-word. Giving every column a floor based on its own longest word
    // means a short column's width no longer depends on how long its neighbor's content is.
    let mut fs = test_font_system();
    let headers = vec![cell("A"), cell("B")];
    let short_rows = vec![vec![cell("short"), cell("moderate length text")]];
    let long_rows = vec![vec![cell("short"), cell("a much, much longer sentence that goes on for quite a while indeed")]];

    let widths_short = md2pdf_layout::test_support::column_widths(&headers, &short_rows, 400.0, &mut fs);
    let widths_long = md2pdf_layout::test_support::column_widths(&headers, &long_rows, 400.0, &mut fs);

    assert!(
        (widths_short[0] - widths_long[0]).abs() < 0.5,
        "column 0's width should not depend on column 1's content length: {} vs {}",
        widths_short[0],
        widths_long[0]
    );
}

#[test]
fn narrow_column_gets_a_fair_share_when_a_sibling_columns_word_is_too_wide_to_fit() {
    // Regression test: when even every column's word-wrap floor doesn't fit in the available
    // width (a genuinely over-constrained table -- e.g. one column's longest "word" is a long
    // unbroken generic-type-like path with no spaces), the old fallback still distributed
    // proportionally to floor size, which squeezed the smaller column down toward the bare
    // MIN_COLUMN_WIDTH_PT clamp since its tiny share of the oversized column's floor rounded
    // down to it. Max-min fair allocation instead satisfies the small column's own (much
    // smaller) floor first and lets only the oversized column absorb the shortfall.
    let mut fs = test_font_system();
    let headers = vec![cell("File"), cell("Type")];
    // No whitespace: this cell is one indivisible "word" as far as measurement is concerned.
    let huge_word = "agirru_engine::SomeVeryLongGenericTypeName<WithGenericParameters,AndMore>";
    let rows = vec![vec![cell("mod.rs"), cell(huge_word)]];

    let widths = md2pdf_layout::test_support::column_widths(&headers, &rows, 200.0, &mut fs);
    assert!(widths[0] > 45.0, "expected the narrow column to get more than the bare minimum width, got {}", widths[0]);
    assert!(widths[1] > widths[0], "expected the oversized column to absorb most of the shortfall");
}
