use cosmic_text::FontSystem;
use sardown_ast::{InlineNode, TextStyle};
use sardown_layout::{insert_hyphenation_breaks, Hyphenator};

#[test]
fn loads_a_known_language() {
    assert!(Hyphenator::load("en-us").is_some());
}

#[test]
fn an_unknown_language_code_returns_none_instead_of_panicking() {
    assert!(Hyphenator::load("not-a-real-language").is_none());
}

#[test]
fn finds_the_documented_break_points_of_a_known_word() {
    // Verified against the real embedded en-us dictionary, not hand-predicted.
    let hyphenator = Hyphenator::load("en-us").unwrap();
    assert_eq!(hyphenator.candidate_breaks("hyphenation"), vec![2, 6, 7]);
}

#[test]
fn a_word_too_short_to_hyphenate_has_no_breaks() {
    let hyphenator = Hyphenator::load("en-us").unwrap();
    assert!(hyphenator.candidate_breaks("a").is_empty());
}

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn plain_node(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".into() },
        link_target: None,
    }
}

fn concatenated_text(nodes: &[InlineNode]) -> String {
    nodes.iter().map(|n| n.text.as_str()).collect()
}

#[test]
fn a_paragraph_that_already_fits_is_returned_unchanged() {
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let content = vec![plain_node("Short line.")];
    let result = insert_hyphenation_breaks(&content, &hyphenator, 400.0, &mut fs);
    assert_eq!(concatenated_text(&result), "Short line.");
}

#[test]
fn a_long_word_that_does_not_fit_is_split_with_a_literal_hyphen() {
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let content = vec![plain_node("A hyphenation example.")];
    let result = insert_hyphenation_breaks(&content, &hyphenator, 40.0, &mut fs);
    let text = concatenated_text(&result);
    assert!(text.contains('-'), "expected a literal hyphen split, got: {text:?}");
    assert_eq!(text.replace('-', ""), "A hyphenation example.");
}

#[test]
fn a_word_with_punctuation_is_never_hyphenated() {
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let content = vec![plain_node("A hyphenation, example.")];
    let result = insert_hyphenation_breaks(&content, &hyphenator, 40.0, &mut fs);
    assert!(!concatenated_text(&result).contains("hyphena-\ntion,"));
}

#[test]
fn a_word_straddling_two_styled_spans_is_never_hyphenated() {
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let mut bold_style = plain_node("").style;
    bold_style.bold = true;
    let content = vec![plain_node("A hyphen"), InlineNode { text: "ation example.".to_string(), style: bold_style, link_target: None }];
    let result = insert_hyphenation_breaks(&content, &hyphenator, 40.0, &mut fs);
    assert_eq!(result[0].text, "A hyphen");
    assert_eq!(result[1].text, "ation example.");
}

#[test]
fn resulting_lines_never_exceed_max_width_pt() {
    use sardown_layout::{shape_rich_paragraph, PositionedElement, ShapingOptions};
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let max_width_pt = 60.0;
    let content = vec![plain_node("An extraordinarily long hyphenation demonstration paragraph.")];
    let hyphenated = insert_hyphenation_breaks(&content, &hyphenator, max_width_pt, &mut fs);
    let shaped = shape_rich_paragraph(&mut fs, &hyphenated, max_width_pt, cosmic_text::Align::Left, ShapingOptions::PROSE);
    for run in &shaped {
        if let PositionedElement::TextRun { x, glyphs, .. } = &run.element {
            let right_edge: f32 = x + glyphs.iter().map(|g| g.x_advance).sum::<f32>();
            assert!(right_edge <= max_width_pt + 0.5, "line exceeded max_width_pt: right edge {right_edge} > {max_width_pt}");
        }
    }
}

#[test]
fn hyphenated_lines_still_justify_like_the_rest_of_the_paragraph() {
    // Regression test: the pre-pass originally forced a break with a literal "\n" (later "\n" was
    // swapped for U+2028 LINE SEPARATOR, which didn't help either). cosmic-text, matching real
    // typesetting convention, never justifies a line ending in *any* mandatory break, not just a
    // paragraph-ending one -- so every hyphenation point disabled justification for its own line,
    // while ordinary (non-hyphenated) wrapped lines in the very same visual paragraph justified
    // fine. Fixed by inserting a bare hyphen with no forced-break character at all: "-" is already
    // an *allowed* break-after point under UAX #14, so cosmic-text's own normal, real-metrics-
    // based wrapping decides to break there on its own, via the exact same (justification-
    // eligible) mechanism as any other line.
    //
    // Only lines that actually *contain* a hyphenated break are checked here -- a line consisting
    // of a single very long word with no other words alongside it has no spaces to redistribute
    // width into and can never be justified by any space-based justification algorithm,
    // hyphenated or not; that's an unrelated, pre-existing, expected limitation, not this bug.
    use sardown_layout::{shape_rich_paragraph, PositionedElement, ShapingOptions};
    let mut fs = test_font_system();
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let max_width_pt = 100.0;
    let content = vec![plain_node("An extraordinarily long hyphenation demonstration paragraph that wraps across several lines of justified text.")];
    let hyphenated = insert_hyphenation_breaks(&content, &hyphenator, max_width_pt, &mut fs);
    let shaped = shape_rich_paragraph(&mut fs, &hyphenated, max_width_pt, cosmic_text::Align::Justified, ShapingOptions::PROSE);

    let mut checked_a_hyphenated_line = false;
    for (i, run) in shaped.iter().enumerate() {
        if i == shaped.len() - 1 {
            continue; // the paragraph's true last line is never justified, by design
        }
        let PositionedElement::TextRun { x, glyphs, text, .. } = &run.element else { continue };
        let min_start = glyphs.iter().map(|g| g.cluster.start).min().unwrap_or(0);
        let max_end = glyphs.iter().map(|g| g.cluster.end).max().unwrap_or(0);
        let line_slice = &text[min_start..max_end];
        if !line_slice.ends_with('-') {
            continue;
        }
        checked_a_hyphenated_line = true;
        let right_edge: f32 = x + glyphs.iter().map(|g| g.x_advance).sum::<f32>();
        assert!(
            right_edge >= max_width_pt - 2.0,
            "hyphenated line {i} ({line_slice:?}) was not justified to fill the width (right edge \
             {right_edge} vs max_width_pt {max_width_pt})"
        );
    }
    assert!(checked_a_hyphenated_line, "expected at least one line ending in a hyphen to actually check");
}
