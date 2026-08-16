use md2pdf_style::{Color, TypographyStyle};

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
