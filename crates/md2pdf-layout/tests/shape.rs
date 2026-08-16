use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};
use md2pdf_layout::{shape_paragraph, shape_rich_paragraph, PositionedElement};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

/// DroidSans.ttf (the primary sans-serif face in these tests) has no glyph for U+2192
/// RIGHTWARDS ARROW; Cantarell-VF.otf does. Loading both, with DroidSans set as the preferred
/// sans-serif family, reproduces real font-fallback substitution deterministically instead of
/// depending on whatever fonts happen to be installed on the machine running the test.
fn font_system_with_fallback() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load primary test font");
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Cantarell-VF.otf")).expect("failed to load fallback test font");
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

#[test]
fn splits_a_run_at_a_font_fallback_boundary_within_one_span() {
    // Regression test: a single plain-text span ("a\u{2192}b", one InlineNode, no bold/italic/
    // color change) used to stay one ShapedRun even when cosmic-text substituted a different
    // font for the arrow character (missing from the primary font). The run's single `font_id`
    // came from whichever glyph was shaped *last*, so glyphs from the other font were rendered
    // against the wrong font's glyph table -- showing an unrelated, effectively random glyph
    // instead of the intended character.
    let mut font_system = font_system_with_fallback();
    let content = vec![plain_run("a\u{2192}b")];
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0);

    let font_ids: Vec<_> = runs
        .iter()
        .map(|r| match &r.element {
            PositionedElement::TextRun { font_id, glyphs, .. } => (*font_id, glyphs.len()),
            other => panic!("expected TextRun, got {other:?}"),
        })
        .collect();

    assert_eq!(font_ids.len(), 3, "expected 3 runs (a / arrow / b split at the font boundary), got {font_ids:?}");
    assert_eq!(font_ids[0].0, font_ids[2].0, "'a' and 'b' should both use the primary font");
    assert_ne!(font_ids[0].0, font_ids[1].0, "the arrow glyph should use a different font than 'a' and 'b'");
    for (_, glyph_count) in &font_ids {
        assert_eq!(*glyph_count, 1, "expected exactly one glyph per single-character run");
    }
}

#[test]
fn shape_rich_paragraph_drops_characters_with_no_font_coverage_instead_of_notdef() {
    // Regression test: cosmic-text falls back to glyph ID 0 (".notdef", the "tofu box") when NO
    // loaded font covers a character. PDF/A strictly forbids emitting that glyph -- krilla's own
    // conformance validation refuses to serialize a document containing one, which previously
    // aborted the ENTIRE render (potentially hundreds of pages of otherwise-fine content) over a
    // single unsupported character somewhere in the source. Dropping just that glyph instead --
    // matching this project's established "skip the one broken piece, don't abort the whole
    // document" convention for images/diagrams/links -- keeps the render succeeding.
    let mut font_system = font_system_with_fallback(); // neither loaded font covers this character
    let content = vec![plain_run("a\u{1D11E}b")]; // U+1D11E MUSICAL SYMBOL G CLEF
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0);

    let all_glyph_ids: Vec<_> = runs
        .iter()
        .flat_map(|r| match &r.element {
            PositionedElement::TextRun { glyphs, .. } => glyphs.iter().map(|g| g.glyph_id).collect::<Vec<_>>(),
            other => panic!("expected TextRun, got {other:?}"),
        })
        .collect();

    assert!(!all_glyph_ids.contains(&0), "expected no .notdef (glyph id 0) in the output, got {all_glyph_ids:?}");
    assert_eq!(all_glyph_ids.len(), 2, "expected 'a' and 'b' to still shape normally, got {all_glyph_ids:?}");
}

#[test]
fn shape_paragraph_drops_characters_with_no_font_coverage_instead_of_notdef() {
    let mut font_system = font_system_with_fallback();
    let content = vec![plain_run("a\u{1D11E}b")];
    let elements = shape_paragraph(&mut font_system, &content, 400.0);

    let all_glyph_ids: Vec<_> = elements
        .iter()
        .flat_map(|e| match e {
            PositionedElement::TextRun { glyphs, .. } => glyphs.iter().map(|g| g.glyph_id).collect::<Vec<_>>(),
            other => panic!("expected TextRun, got {other:?}"),
        })
        .collect();

    assert!(!all_glyph_ids.contains(&0), "expected no .notdef (glyph id 0) in the output, got {all_glyph_ids:?}");
    assert_eq!(all_glyph_ids.len(), 2, "expected 'a' and 'b' to still shape normally, got {all_glyph_ids:?}");
}
