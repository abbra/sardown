use md2pdf_style::{Color, Stylesheet, TextAlignment, VerticalAlign};

#[test]
fn default_slides_style_has_no_default_layout_and_a_half_scale_floor() {
    let sheet = Stylesheet::default();
    assert_eq!(sheet.slides.default_layout, None);
    assert_eq!(sheet.slides.min_scale, 0.5);
    assert!(sheet.slides.layouts.is_empty());
}

#[test]
fn a_named_layout_deserializes_its_overrides() {
    let toml_text = r##"
        [slides]
        default_layout = "title"

        [slides.layouts.title]
        alignment = "center"
        vertical_align = "center"
        body_size_pt = 20.0
        background_color = "#1b0d33"
        text_color = "#ffffff"
        suppress_header = true
        suppress_footer = true
    "##;
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    assert_eq!(sheet.slides.default_layout, Some("title".to_string()));
    let layout = sheet.slides.layouts.get("title").unwrap();
    assert_eq!(layout.alignment, Some(TextAlignment::Center));
    assert_eq!(layout.vertical_align, VerticalAlign::Center);
    assert_eq!(layout.body_size_pt, Some(20.0));
    assert_eq!(layout.background_color, Some(Color([27, 13, 51])));
    assert_eq!(layout.text_color, Some(Color([255, 255, 255])));
    assert!(layout.suppress_header);
    assert!(layout.suppress_footer);
}

#[test]
fn a_layout_with_no_fields_set_leaves_every_override_absent() {
    let toml_text = "[slides.layouts.content]\n";
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = sheet.slides.layouts.get("content").unwrap();
    assert_eq!(layout.alignment, None);
    assert_eq!(layout.vertical_align, VerticalAlign::Top);
    assert_eq!(layout.body_size_pt, None);
    assert_eq!(layout.background_color, None);
    assert_eq!(layout.text_color, None);
    assert!(!layout.suppress_header);
    assert!(!layout.suppress_footer);
}

#[test]
fn default_layout_naming_an_undefined_layout_is_a_validation_error() {
    let path = std::env::temp_dir().join("md2pdf-test-slides-bad-default-layout.toml");
    std::fs::write(&path, "[slides]\ndefault_layout = \"missing\"\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("missing"), "expected the error to name the missing layout, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn default_layout_naming_a_defined_layout_loads_successfully() {
    let path = std::env::temp_dir().join("md2pdf-test-slides-good-default-layout.toml");
    std::fs::write(&path, "[slides]\ndefault_layout = \"content\"\n\n[slides.layouts.content]\n").unwrap();
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.slides.default_layout, Some("content".to_string()));
    std::fs::remove_file(&path).unwrap();
}
