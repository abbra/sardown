use md2pdf_style::{NumberingFormat, PageFormat, PageNumbering, PageStyle};

#[test]
fn default_page_style_is_letter_with_todays_current_margin() {
    let page = PageStyle::default();
    assert_eq!(page.format, PageFormat::Letter);
    assert_eq!(page.dimensions_mm(), (215.9, 279.4));
    assert_eq!(page.margin_mm, 25.4);
}

#[test]
fn every_named_format_has_its_standard_dimensions() {
    assert_eq!(PageFormat::Letter.dimensions_mm(), (215.9, 279.4));
    assert_eq!(PageFormat::A4.dimensions_mm(), (210.0, 297.0));
    assert_eq!(PageFormat::A3.dimensions_mm(), (297.0, 420.0));
    assert_eq!(PageFormat::A5.dimensions_mm(), (148.0, 210.0));
    assert_eq!(PageFormat::Legal.dimensions_mm(), (215.9, 355.6));
}

#[test]
fn explicit_width_and_height_override_the_format_preset() {
    let page = PageStyle {
        format: PageFormat::A4,
        width_mm: Some(100.0),
        height_mm: Some(50.0),
        margin_mm: 10.0,
        inner_margin_mm: None,
        outer_margin_mm: None,
        numbering: PageNumbering::default(),
    };
    assert_eq!(page.dimensions_mm(), (100.0, 50.0));
}

#[test]
fn deserializes_named_formats_as_lowercase_strings() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        format: PageFormat,
    }
    let parsed: Wrapper = toml::from_str(r#"format = "a4""#).unwrap();
    assert_eq!(parsed.format, PageFormat::A4);
}

#[test]
fn a_toml_file_setting_only_margin_mm_keeps_every_other_page_field_default() {
    let page: PageStyle = toml::from_str("margin_mm = 30.0").unwrap();
    assert_eq!(page.format, PageFormat::Letter);
    assert_eq!(page.width_mm, None);
    assert_eq!(page.height_mm, None);
    assert_eq!(page.margin_mm, 30.0);
}

#[test]
fn default_page_style_includes_default_numbering() {
    let page = PageStyle::default();
    assert_eq!(page.numbering.format, NumberingFormat::Arabic);
    assert_eq!(page.numbering.start_at, 1);
}

#[test]
fn a_nested_numbering_table_composes_with_the_rest_of_page_style() {
    let toml_text = "margin_mm = 30.0\n[numbering]\nstart_at = 5\n";
    let page: PageStyle = toml::from_str(toml_text).unwrap();
    assert_eq!(page.margin_mm, 30.0);
    assert_eq!(page.format, PageFormat::Letter);
    assert_eq!(page.numbering.start_at, 5);
    assert_eq!(page.numbering.format, NumberingFormat::Arabic);
}
