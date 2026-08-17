use md2pdf_style::{Color, HeaderFooterMode, HeaderFooterStyle, HeaderZones};

#[test]
fn default_header_zones_are_all_empty() {
    let zones = HeaderZones::default();
    assert_eq!(zones.left, "");
    assert_eq!(zones.center, "");
    assert_eq!(zones.right, "");
}

#[test]
fn header_footer_mode_default_is_uniform() {
    assert_eq!(HeaderFooterMode::default(), HeaderFooterMode::Uniform);
}

#[test]
fn deserializes_two_sided_mode_as_a_snake_case_string() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        mode: HeaderFooterMode,
    }
    assert_eq!(toml::from_str::<Wrapper>(r#"mode = "two_sided""#).unwrap().mode, HeaderFooterMode::TwoSided);
    assert_eq!(toml::from_str::<Wrapper>(r#"mode = "uniform""#).unwrap().mode, HeaderFooterMode::Uniform);
}

#[test]
fn header_zones_deserializes_from_a_partial_toml_table() {
    let zones: HeaderZones = toml::from_str(r#"center = "{h1}""#).unwrap();
    assert_eq!(zones.left, "");
    assert_eq!(zones.center, "{h1}");
    assert_eq!(zones.right, "");
}

#[test]
fn default_header_footer_style_is_disabled_with_sensible_defaults() {
    let style = HeaderFooterStyle::default();
    assert!(!style.enabled);
    assert_eq!(style.font_family, "sans-serif");
    assert_eq!(style.font_size_pt, 9.0);
    assert_eq!(style.color, Color([102, 102, 102]));
    assert_eq!(style.mode, HeaderFooterMode::Uniform);
    assert!(style.suppress_on_chapter_start);
    assert_eq!(style.uniform.center, "");
    assert_eq!(style.odd.center, "");
    assert_eq!(style.even.center, "");
}

#[test]
fn a_partial_toml_overrides_only_the_fields_it_sets() {
    let style: HeaderFooterStyle = toml::from_str("enabled = true\nfont_size_pt = 10.0").unwrap();
    assert!(style.enabled);
    assert_eq!(style.font_size_pt, 10.0);
    assert_eq!(style.font_family, "sans-serif");
    assert_eq!(style.color, Color([102, 102, 102]));
}

#[test]
fn deserializes_uniform_zone_content() {
    let toml_text = "enabled = true\n[uniform]\nleft = \"{h1}\"\nright = \"Page {page}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert_eq!(style.uniform.left, "{h1}");
    assert_eq!(style.uniform.right, "Page {page}");
    assert_eq!(style.uniform.center, "");
}

#[test]
fn deserializes_odd_and_even_zone_content_for_two_sided_mode() {
    let toml_text = "enabled = true\nmode = \"two_sided\"\n[odd]\nleft = \"{h1}\"\n[even]\nleft = \"Page {page}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert_eq!(style.mode, HeaderFooterMode::TwoSided);
    assert_eq!(style.odd.left, "{h1}");
    assert_eq!(style.even.left, "Page {page}");
}

#[test]
fn validate_accepts_a_style_with_only_known_placeholders() {
    let toml_text = "enabled = true\n[uniform]\nleft = \"{h1}\"\ncenter = \"static text\"\nright = \"Page {page} of {total_pages}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert!(style.validate("header").is_ok());
}

#[test]
fn validate_accepts_the_title_and_author_placeholders() {
    let toml_text = "enabled = true\n[uniform]\nleft = \"{title}\"\nright = \"{author}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert!(style.validate("header").is_ok());
}

#[test]
fn validate_rejects_an_unknown_placeholder() {
    let toml_text = "enabled = true\n[uniform]\nleft = \"{bogus}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    let err = style.validate("header").unwrap_err();
    assert!(format!("{err:?}").contains("bogus"), "expected the bad placeholder name in the error, got: {err:?}");
}

#[test]
fn validate_rejects_an_unterminated_brace() {
    let toml_text = "enabled = true\n[uniform]\nleft = \"{h1\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert!(style.validate("header").is_err());
}

#[test]
fn validate_checks_odd_and_even_zones_too() {
    let toml_text = "enabled = true\nmode = \"two_sided\"\n[even]\nright = \"{nonsense}\"\n";
    let style: HeaderFooterStyle = toml::from_str(toml_text).unwrap();
    assert!(style.validate("footer").is_err());
}
