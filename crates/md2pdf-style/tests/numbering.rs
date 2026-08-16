use md2pdf_style::{NumberingFormat, PageNumbering};

#[test]
fn default_page_numbering_is_arabic_starting_at_one() {
    let numbering = PageNumbering::default();
    assert_eq!(numbering.format, NumberingFormat::Arabic);
    assert_eq!(numbering.start_at, 1);
}

#[test]
fn deserializes_named_formats_as_snake_case_strings() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        format: NumberingFormat,
    }
    assert_eq!(toml::from_str::<Wrapper>(r#"format = "arabic""#).unwrap().format, NumberingFormat::Arabic);
    assert_eq!(toml::from_str::<Wrapper>(r#"format = "roman_lower""#).unwrap().format, NumberingFormat::RomanLower);
    assert_eq!(toml::from_str::<Wrapper>(r#"format = "roman_upper""#).unwrap().format, NumberingFormat::RomanUpper);
}

#[test]
fn a_partial_override_keeps_the_other_field_default() {
    let numbering: PageNumbering = toml::from_str("start_at = 5").unwrap();
    assert_eq!(numbering.start_at, 5);
    assert_eq!(numbering.format, NumberingFormat::Arabic);
}
