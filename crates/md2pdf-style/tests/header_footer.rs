use md2pdf_style::{HeaderFooterMode, HeaderZones};

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
