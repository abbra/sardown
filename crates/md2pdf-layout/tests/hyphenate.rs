use md2pdf_layout::Hyphenator;

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