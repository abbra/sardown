//! The thread-local shaping caches (`WORD_SHAPING_CACHE`, `MONOSPACE_ADVANCE_CACHE`,
//! `FAMILY_KNOWN_CACHE`) memoize shaping results whose correctness domain is ONE font
//! database, not the whole thread. These tests pin that domain: two documents rendered
//! sequentially on one thread, with different loaded fonts behind the identical style key,
//! must each get their own metrics.

use cosmic_text::FontSystem;
use sardown_ast::{InlineNode, TextStyle};
use sardown_layout::{insert_hyphenation_breaks, shape_paragraph, Hyphenator, PositionedElement};

const PROBE_TEXT: &str = "hyphenation";
const PROBE_SIZE: f32 = 12.0;
fn droid_only_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load DroidSans.ttf");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn cantarell_only_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Cantarell-VF.otf")).expect("failed to load Cantarell-VF.otf");
    db.set_sans_serif_family("Cantarell");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

/// Both systems resolve the *same* generic keyword, so the cache key
/// `(family="sans-serif", size, bold, italic)` collides across them even though the
/// underlying face -- and therefore every advance -- differs.
fn plain_node(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: PROBE_SIZE, color: [0, 0, 0], font_family: "sans-serif".into() },
        link_target: None,
    }
}

/// Natural (unwrapped) shaped width of [`PROBE_TEXT`] in points under one cold system,
/// measured on a pristine thread so no earlier test's cache entries can leak in.
fn fresh_natural_width(build: fn() -> FontSystem) -> f32 {
    std::thread::spawn(move || {
        let mut fs = build();
        shape_paragraph(&mut fs, &[plain_node(PROBE_TEXT)], f32::MAX)
            .iter()
            .map(|e| match e {
                PositionedElement::TextRun { glyphs, .. } => glyphs.iter().map(|g| g.x_advance).sum::<f32>(),
                _ => 0.0,
            })
            .sum()
    })
    .join()
    .unwrap()
}

/// Byte offset of the first inserted hyphenation break in [`PROBE_TEXT`] under `fs`
/// (`None` when the sentence fits `max_width_pt` without splitting).
fn split_offset(font_system: &mut FontSystem, max_width_pt: f32) -> Option<usize> {
    let hyphenator = Hyphenator::load("en-us").unwrap();
    let content = vec![plain_node(PROBE_TEXT)];
    let result = insert_hyphenation_breaks(&content, &hyphenator, max_width_pt, font_system);
    let text: String = result.iter().map(|n| n.text.as_str()).collect();
    text.find('-')
}

/// Same as [`fresh_natural_width`] but for the hyphenation pass: pristine thread, so the
/// measurement is the ground truth for that font system alone.
fn fresh_split_offset(build: fn() -> FontSystem, max_width_pt: f32) -> Option<usize> {
    std::thread::spawn(move || {
        let mut fs = build();
        split_offset(&mut fs, max_width_pt)
    })
    .join()
    .unwrap()
}

#[test]
fn calibration_fonts_differ() {
    let droid = fresh_natural_width(droid_only_system);
    let cantarell = fresh_natural_width(cantarell_only_system);
    assert!((droid - cantarell).abs() > 1.0, "fixture fonts unexpectedly agree ({droid} vs {cantarell}); this test's discriminator is void");
}

/// Strictly between the two fixture fonts' natural widths for [`PROBE_TEXT`] (measured:
/// Droid Sans 68.05pt, Cantarell 66.40pt), so Cantarell fits the line unsplit while Droid
/// Sans must hyphenate at its largest fitting dictionary break. Fixture fonts are
/// deterministic, and so is this threshold.
const DISCRIMINATOR_WIDTH_PT: f32 = 67.2;

#[test]
fn fresh_systems_disagree_at_the_discriminator_width() {
    // Guards the discriminator itself: if either font's behavior drifts (different fixture,
    // cosmic-text change), this fails loudly instead of letting the staleness tests pass vacuously.
    let droid = fresh_split_offset(droid_only_system, DISCRIMINATOR_WIDTH_PT);
    let cantarell = fresh_split_offset(cantarell_only_system, DISCRIMINATOR_WIDTH_PT);
    assert!(droid.is_some(), "expected Droid Sans to need hyphenation at {}pt", DISCRIMINATOR_WIDTH_PT);
    assert!(cantarell.is_none(), "expected Cantarell to fit without hyphenation at {}pt", DISCRIMINATOR_WIDTH_PT);
}

#[test]
fn a_second_font_system_on_the_same_thread_is_shaped_with_its_own_fonts() {
    let mut fs = droid_only_system();
    let first_document = split_offset(&mut fs, DISCRIMINATOR_WIDTH_PT);
    drop(fs);
    let mut fs = cantarell_only_system();
    let second_document = split_offset(&mut fs, DISCRIMINATOR_WIDTH_PT);
    drop(fs);

    assert_eq!(first_document, fresh_split_offset(droid_only_system, DISCRIMINATOR_WIDTH_PT), "first document diverges from its own ground truth");
    assert_eq!(
        second_document,
        fresh_split_offset(cantarell_only_system, DISCRIMINATOR_WIDTH_PT),
        "second document reused the first document's cached metrics (stale thread-local shaping cache)"
    );
    assert_ne!(first_document, second_document);
}

#[test]
fn document_order_does_not_change_shaping() {
    let mut fs = cantarell_only_system();
    let first_document = split_offset(&mut fs, DISCRIMINATOR_WIDTH_PT);
    drop(fs);
    let mut fs = droid_only_system();
    let second_document = split_offset(&mut fs, DISCRIMINATOR_WIDTH_PT);
    drop(fs);

    assert_eq!(first_document, fresh_split_offset(cantarell_only_system, DISCRIMINATOR_WIDTH_PT), "first document diverges from its own ground truth");
    assert_eq!(
        second_document,
        fresh_split_offset(droid_only_system, DISCRIMINATOR_WIDTH_PT),
        "second document reused the first document's cached metrics (stale thread-local shaping cache)"
    );
    assert_ne!(first_document, second_document);
}
