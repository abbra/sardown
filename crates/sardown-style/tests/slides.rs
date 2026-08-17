use sardown_style::{Color, ImageCorner, Stylesheet, TextAlignment, VerticalAlign};

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
    assert_eq!(layout.secondary_text_color, None);
    assert!(!layout.suppress_header);
    assert!(!layout.suppress_footer);
    assert!(layout.background_images.is_empty());
}

#[test]
fn a_layout_can_configure_a_secondary_text_color() {
    let toml_text = "[slides.layouts.title]\ntext_color = \"#ffffff\"\nsecondary_text_color = \"#9b8ab4\"\n";
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = sheet.slides.layouts.get("title").unwrap();
    assert_eq!(layout.text_color, Some(Color([255, 255, 255])));
    assert_eq!(layout.secondary_text_color, Some(Color([155, 138, 180])));
}

#[test]
fn a_layout_can_configure_a_background_image() {
    let toml_text = "[[slides.layouts.title.background_images]]\npath = \"logo.png\"\ncorner = \"top_right\"\nwidth_pt = 80.0\nmargin_pt = 20.0\n";
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = sheet.slides.layouts.get("title").unwrap();
    assert_eq!(layout.background_images.len(), 1);
    let image = &layout.background_images[0];
    assert_eq!(image.path, std::path::PathBuf::from("logo.png"));
    assert_eq!(image.corner, ImageCorner::TopRight);
    assert_eq!(image.width_pt, 80.0);
    assert_eq!(image.margin_pt, 20.0);
}

#[test]
fn a_background_image_with_no_corner_width_or_margin_gets_the_documented_defaults() {
    let toml_text = "[[slides.layouts.title.background_images]]\npath = \"logo.png\"\n";
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    let image = &sheet.slides.layouts.get("title").unwrap().background_images[0];
    assert_eq!(image.corner, ImageCorner::BottomLeft);
    assert_eq!(image.width_pt, 60.0);
    assert_eq!(image.margin_pt, 14.0);
}

#[test]
fn a_layout_can_configure_multiple_background_images() {
    let toml_text = "[[slides.layouts.title.background_images]]\npath = \"logo.png\"\ncorner = \"top_left\"\n\n[[slides.layouts.title.background_images]]\npath = \"logo.svg\"\ncorner = \"bottom_right\"\n";
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    let images = &sheet.slides.layouts.get("title").unwrap().background_images;
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].path, std::path::PathBuf::from("logo.png"));
    assert_eq!(images[0].corner, ImageCorner::TopLeft);
    assert_eq!(images[1].path, std::path::PathBuf::from("logo.svg"));
    assert_eq!(images[1].corner, ImageCorner::BottomRight);
}

#[test]
fn a_background_image_missing_its_path_is_a_deserialization_error() {
    let toml_text = "[[slides.layouts.title.background_images]]\ncorner = \"top_right\"\n";
    assert!(toml::from_str::<Stylesheet>(toml_text).is_err());
}

#[test]
fn default_layout_naming_an_undefined_layout_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-slides-bad-default-layout.toml");
    std::fs::write(&path, "[slides]\ndefault_layout = \"missing\"\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("missing"), "expected the error to name the missing layout, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn default_layout_naming_a_defined_layout_loads_successfully() {
    let path = std::env::temp_dir().join("sardown-test-slides-good-default-layout.toml");
    std::fs::write(&path, "[slides]\ndefault_layout = \"content\"\n\n[slides.layouts.content]\n").unwrap();
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.slides.default_layout, Some("content".to_string()));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn a_zero_min_scale_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-slides-zero-min-scale.toml");
    std::fs::write(&path, "[slides]\nmin_scale = 0.0\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("min_scale"), "expected the error to name min_scale, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn a_negative_min_scale_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-slides-negative-min-scale.toml");
    std::fs::write(&path, "[slides]\nmin_scale = -0.5\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("min_scale"), "expected the error to name min_scale, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn a_min_scale_above_one_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-slides-min-scale-above-one.toml");
    std::fs::write(&path, "[slides]\nmin_scale = 1.5\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("min_scale"), "expected the error to name min_scale, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn a_min_scale_of_exactly_one_loads_successfully() {
    let path = std::env::temp_dir().join("sardown-test-slides-min-scale-one.toml");
    std::fs::write(&path, "[slides]\nmin_scale = 1.0\n").unwrap();
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.slides.min_scale, 1.0);
    std::fs::remove_file(&path).unwrap();
}
