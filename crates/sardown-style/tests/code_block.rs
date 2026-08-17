use sardown_style::{CodeBlockStyle, Color, LabelStyle};

#[test]
fn default_code_block_style_matches_todays_hardcoded_background_and_font() {
    let code_block = CodeBlockStyle::default();
    assert_eq!(code_block.syntax_theme, "InspiredGitHub");
    assert_eq!(code_block.label_style, LabelStyle::None);
    assert_eq!(code_block.default_label, "text");
    assert_eq!(code_block.default.background, Color([245, 245, 245]));
    assert_eq!(code_block.default.font_family, "monospace");
    assert_eq!(code_block.default.font_size_pt, 10.0);
}

#[test]
fn resolve_with_no_language_uses_the_default_label_and_default_style() {
    let code_block = CodeBlockStyle::default();
    let resolved = code_block.resolve(None);
    assert_eq!(resolved.label, "text");
    assert_eq!(resolved.background, Color([245, 245, 245]));
}

#[test]
fn resolve_with_an_unconfigured_language_title_cases_the_token() {
    let code_block = CodeBlockStyle::default();
    let resolved = code_block.resolve(Some("rust"));
    assert_eq!(resolved.label, "Rust");
    assert_eq!(resolved.background, Color([245, 245, 245]), "falls back to [code_block.default]");
}

#[test]
fn resolve_uses_an_explicit_per_language_label_when_set() {
    let toml_text = r##"
        [languages.python]
        label = "Python 3"
        background = "#f0f8ff"
    "##;
    let code_block: CodeBlockStyle = toml::from_str(toml_text).unwrap();
    let resolved = code_block.resolve(Some("python"));
    assert_eq!(resolved.label, "Python 3");
    assert_eq!(resolved.background, Color([240, 248, 255]));
    assert_eq!(resolved.font_family, "monospace", "unset field falls back to [code_block.default]");
}

#[test]
fn resolve_title_cases_the_token_when_the_language_section_exists_but_sets_no_label() {
    let toml_text = r##"
        [languages.rust]
        background = "#fdf6e3"
    "##;
    let code_block: CodeBlockStyle = toml::from_str(toml_text).unwrap();
    let resolved = code_block.resolve(Some("rust"));
    assert_eq!(resolved.label, "Rust");
    assert_eq!(resolved.background, Color([253, 246, 227]));
}

#[test]
fn deserializes_each_named_label_style() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        label_style: LabelStyle,
    }
    assert_eq!(toml::from_str::<Wrapper>(r#"label_style = "none""#).unwrap().label_style, LabelStyle::None);
    assert_eq!(toml::from_str::<Wrapper>(r#"label_style = "corner""#).unwrap().label_style, LabelStyle::Corner);
    assert_eq!(toml::from_str::<Wrapper>(r#"label_style = "header_bar""#).unwrap().label_style, LabelStyle::HeaderBar);
    assert_eq!(toml::from_str::<Wrapper>(r#"label_style = "inline""#).unwrap().label_style, LabelStyle::Inline);
}
