use sardown_style::{NumberingFormat, PageNumbering};

#[test]
fn default_page_numbering_is_arabic_starting_at_one() {
    let numbering = PageNumbering::default();
    assert_eq!(numbering.format, NumberingFormat::Arabic);
    assert_eq!(numbering.start_at, 1);
    assert!(numbering.resets.is_empty());
}

#[test]
fn deserializes_a_reset_with_an_explicit_format_and_start_at() {
    let toml_text = "[[resets]]\nat_heading = \"chapter-one\"\nformat = \"arabic\"\nstart_at = 1\n";
    let numbering: PageNumbering = toml::from_str(toml_text).unwrap();
    assert_eq!(numbering.resets.len(), 1);
    assert_eq!(numbering.resets[0].at_heading, "chapter-one");
    assert_eq!(numbering.resets[0].format, NumberingFormat::Arabic);
    assert_eq!(numbering.resets[0].start_at, 1);
}

#[test]
fn a_reset_with_only_at_heading_set_defaults_format_and_start_at() {
    let toml_text = "[[resets]]\nat_heading = \"chapter-one\"\n";
    let numbering: PageNumbering = toml::from_str(toml_text).unwrap();
    assert_eq!(numbering.resets[0].format, NumberingFormat::Arabic);
    assert_eq!(numbering.resets[0].start_at, 1);
}

#[test]
fn multiple_resets_deserialize_in_document_order() {
    let toml_text = "[[resets]]\nat_heading = \"preface\"\nformat = \"roman_lower\"\n\n[[resets]]\nat_heading = \"chapter-one\"\nformat = \"arabic\"\n";
    let numbering: PageNumbering = toml::from_str(toml_text).unwrap();
    assert_eq!(numbering.resets.len(), 2);
    assert_eq!(numbering.resets[0].at_heading, "preface");
    assert_eq!(numbering.resets[1].at_heading, "chapter-one");
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
