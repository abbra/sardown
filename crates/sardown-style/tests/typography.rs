use sardown_style::{Color, TextAlignment, TypographyStyle};

#[test]
fn default_typography_matches_todays_hardcoded_body_text() {
    let typography = TypographyStyle::default();
    assert_eq!(typography.font_family, "sans-serif");
    assert!(typography.font_dirs.is_empty());
    assert!(typography.use_system_fonts);
    assert_eq!(typography.body_size_pt, 12.0);
    assert_eq!(typography.body_color, Color([0, 0, 0]));
}

#[test]
fn a_partial_toml_overrides_only_the_fields_it_sets() {
    let typography: TypographyStyle = toml::from_str(r#"body_size_pt = 11.0"#).unwrap();
    assert_eq!(typography.body_size_pt, 11.0);
    assert_eq!(typography.font_family, "sans-serif");
    assert_eq!(typography.body_color, Color([0, 0, 0]));
}

#[test]
fn font_dirs_deserializes_a_list_of_paths() {
    let typography: TypographyStyle = toml::from_str(r#"font_dirs = ["/opt/fonts", "vendor/fonts"]"#).unwrap();
    assert_eq!(typography.font_dirs, vec![std::path::PathBuf::from("/opt/fonts"), std::path::PathBuf::from("vendor/fonts")]);
}

#[test]
fn default_typography_uses_left_alignment() {
    assert_eq!(TypographyStyle::default().alignment, TextAlignment::Left);
}

#[test]
fn a_toml_document_can_request_justified_alignment() {
    let typography: TypographyStyle = toml::from_str(r#"alignment = "justify""#).unwrap();
    assert_eq!(typography.alignment, TextAlignment::Justify);
}

#[test]
fn default_typography_has_hyphenation_disabled_with_english_us_as_the_default_language() {
    let typography = TypographyStyle::default();
    assert!(!typography.hyphenation);
    assert_eq!(typography.language, "en-us");
}

#[test]
fn a_toml_document_can_enable_hyphenation_and_set_a_language() {
    let typography: TypographyStyle = toml::from_str("hyphenation = true\nlanguage = \"de-1996\"\n").unwrap();
    assert!(typography.hyphenation);
    assert_eq!(typography.language, "de-1996");
}
