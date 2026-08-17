use cosmic_text::{Align, FontSystem};
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
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn colored_run(text: &str, color: [u8; 3]) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color, font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn font_system_with_distinct_generic_families() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load primary test font");
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Cantarell-VF.otf")).expect("failed to load secondary test font");
    db.set_sans_serif_family("Droid Sans");
    db.set_monospace_family("Cantarell");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn run_with_family(text: &str, font_family: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: font_family.to_string() },
        link_target: None,
    }
}

#[test]
fn shape_paragraph_resolves_distinct_generic_keywords_to_distinct_fonts() {
    let mut font_system = font_system_with_distinct_generic_families();

    let sans_elements = shape_paragraph(&mut font_system, &[run_with_family("Hello", "sans-serif")], 400.0);
    let mono_elements = shape_paragraph(&mut font_system, &[run_with_family("Hello", "monospace")], 400.0);

    let font_id_of = |elements: &[PositionedElement]| match &elements[0] {
        PositionedElement::TextRun { font_id, .. } => *font_id,
        other => panic!("expected TextRun, got {other:?}"),
    };
    assert_ne!(font_id_of(&sans_elements), font_id_of(&mono_elements), "expected \"sans-serif\" and \"monospace\" to resolve to different fonts");
}

#[test]
fn shape_paragraph_resolves_a_literal_family_name_that_is_actually_loaded() {
    let mut font_system = font_system_with_distinct_generic_families();

    let droid_elements = shape_paragraph(&mut font_system, &[run_with_family("Hello", "Droid Sans")], 400.0);
    let cantarell_elements = shape_paragraph(&mut font_system, &[run_with_family("Hello", "Cantarell")], 400.0);

    let font_id_of = |elements: &[PositionedElement]| match &elements[0] {
        PositionedElement::TextRun { font_id, .. } => *font_id,
        other => panic!("expected TextRun, got {other:?}"),
    };
    assert_ne!(font_id_of(&droid_elements), font_id_of(&cantarell_elements), "expected two distinct literal family names to resolve to different fonts");
}

#[test]
fn shape_paragraph_falls_back_to_sans_serif_for_an_unknown_family_name_without_panicking() {
    let mut font_system = font_system_with_distinct_generic_families();
    let elements = shape_paragraph(&mut font_system, &[run_with_family("Hello", "Not A Real Font Xyz")], 400.0);

    match &elements[0] {
        PositionedElement::TextRun { glyphs, .. } => assert!(!glyphs.is_empty(), "expected shaping to still succeed via the sans-serif fallback"),
        other => panic!("expected TextRun, got {other:?}"),
    }
}

#[test]
fn shape_rich_paragraph_mixes_two_font_families_within_one_call() {
    let mut font_system = font_system_with_distinct_generic_families();
    let content = vec![run_with_family("sans ", "sans-serif"), run_with_family("mono", "monospace")];
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0, Align::Left);

    let font_ids: Vec<_> = runs
        .iter()
        .map(|r| match &r.element {
            PositionedElement::TextRun { font_id, .. } => *font_id,
            other => panic!("expected TextRun, got {other:?}"),
        })
        .collect();
    assert_eq!(font_ids.len(), 2, "expected one ShapedRun per span since they use different fonts, got {font_ids:?}");
    assert_ne!(font_ids[0], font_ids[1], "expected the two spans' distinct font_family values to resolve to different fonts");
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
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0, Align::Left);

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
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0, Align::Left);

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

#[test]
fn shape_rich_paragraph_keeps_correct_span_colors_across_embedded_newlines() {
    // Regression test: cosmic-text splits text passed to set_rich_text into separate internal
    // "BufferLine"s at each '\n', and LayoutGlyph::start/end reset to 0 at the start of every
    // such line -- they are NOT a running offset into the whole rich-text sequence, despite
    // looking like one for single-line content. Comparing that line-relative offset directly
    // against globally-accumulated span ranges (the pre-fix behavior) made every line after the
    // first inherit whichever span happened to occupy that same *relative* byte position on line
    // 0 -- exactly the combination a syntax-highlighted, multi-line code block produces (many
    // spans from per-token highlighting, several embedded newlines from syntect's own per-line
    // tokenization), and exactly why it went uncaught by every earlier wrapped-paragraph test
    // (which never mixed multiple spans with an embedded newline in one shape_rich_paragraph call).
    let content = vec![
        colored_run("aaa", [255, 0, 0]), // line 0, global byte range 0..3
        colored_run("\n", [255, 0, 0]),  // line 0's own newline
        colored_run("bbb", [0, 255, 0]), // line 1, global byte range 4..7, but LOCAL offset 0
                                         // on its own BufferLine
    ];
    let mut font_system = test_font_system();
    let runs = shape_rich_paragraph(&mut font_system, &content, 400.0, Align::Left);

    let colors_by_source_index: std::collections::HashMap<usize, [u8; 3]> = runs
        .iter()
        .map(|r| match &r.element {
            PositionedElement::TextRun { color, .. } => (r.source_index, *color),
            other => panic!("expected TextRun, got {other:?}"),
        })
        .collect();

    assert_eq!(colors_by_source_index.get(&0), Some(&[255, 0, 0]), "expected 'aaa' (span 0) to keep its own red");
    assert_eq!(
        colors_by_source_index.get(&2),
        Some(&[0, 255, 0]),
        "expected 'bbb' (span 2, on the second BufferLine) to use its own green, not span 0's color"
    );
}

#[test]
fn shape_rich_paragraph_justifies_a_non_last_wrapped_line_to_the_full_width() {
    let mut font_system = test_font_system();
    // Long enough to wrap onto at least 2 lines at 200pt.
    let content = vec![plain_run("one two three four five six seven eight nine ten")];
    let max_width_pt = 200.0;

    let left_runs = shape_rich_paragraph(&mut font_system, &content, max_width_pt, Align::Left);
    let justified_runs = shape_rich_paragraph(&mut font_system, &content, max_width_pt, Align::Justified);

    let rightmost_extent_of_first_line = |runs: &[md2pdf_layout::ShapedRun]| -> f32 {
        let first_line_y = match &runs[0].element {
            PositionedElement::TextRun { y, .. } => *y,
            other => panic!("expected TextRun, got {other:?}"),
        };
        runs.iter()
            .filter_map(|r| match &r.element {
                PositionedElement::TextRun { y, glyphs, x, .. } if *y == first_line_y => Some(x + glyphs.iter().map(|g| g.x_advance).sum::<f32>()),
                _ => None,
            })
            .fold(0.0f32, f32::max)
    };

    let left_extent = rightmost_extent_of_first_line(&left_runs);
    let justified_extent = rightmost_extent_of_first_line(&justified_runs);
    assert!(
        justified_extent > left_extent + 1.0,
        "expected the justified first (non-last) line to reach further right ({justified_extent}) than the left-aligned one ({left_extent})"
    );
    assert!(
        (justified_extent - max_width_pt).abs() < 2.0,
        "expected the justified line to reach very close to the {max_width_pt}pt wrap width, got {justified_extent}"
    );
}
