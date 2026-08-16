use md2pdf_style::{Color, HeadingStyle};

#[test]
fn default_heading_style_matches_todays_hardcoded_sizes_and_color() {
    let heading = HeadingStyle::default();
    assert_eq!(heading.space_before_factor, 0.8);
    let expected_sizes = [28.0, 22.0, 18.0, 16.0, 14.0, 12.0];
    for (level, &expected_size) in (1u8..=6).zip(expected_sizes.iter()) {
        let resolved = heading.resolve(level);
        assert_eq!(resolved.size_pt, expected_size, "level {level}");
        assert_eq!(resolved.color, Color([0, 0, 0]), "level {level}");
        assert_eq!(resolved.font_family, "sans-serif", "level {level}");
    }
}

#[test]
fn a_per_level_override_only_changes_that_level() {
    let toml_text = r##"
        [levels.2]
        size_pt = 30.0
        color = "#ff0000"
    "##;
    let heading: HeadingStyle = toml::from_str(toml_text).unwrap();

    let level_2 = heading.resolve(2);
    assert_eq!(level_2.size_pt, 30.0);
    assert_eq!(level_2.color, Color([255, 0, 0]));
    assert_eq!(level_2.font_family, "sans-serif", "unset field falls back to [heading]'s own default");

    let level_1 = heading.resolve(1);
    assert_eq!(level_1.size_pt, 28.0, "level 1 must be untouched by level 2's override");
}

#[test]
fn a_document_wide_color_override_applies_to_every_level_without_its_own_override() {
    let heading: HeadingStyle = toml::from_str(r##"color = "#333333""##).unwrap();
    for level in 1u8..=6 {
        assert_eq!(heading.resolve(level).color, Color([51, 51, 51]), "level {level}");
    }
}
