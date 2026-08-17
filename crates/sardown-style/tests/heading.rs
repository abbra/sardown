use sardown_style::{Color, HeadingStyle};

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
        assert_eq!(resolved.underline_width_pt, 0.0, "level {level}: no underline by default");
    }
}

#[test]
fn a_document_wide_underline_applies_to_every_level_without_its_own_override() {
    let heading: HeadingStyle = toml::from_str(
        r##"underline_width_pt = 2.0
underline_color = "#d2d2d2""##,
    )
    .unwrap();
    for level in 1u8..=6 {
        let resolved = heading.resolve(level);
        assert_eq!(resolved.underline_width_pt, 2.0, "level {level}");
        assert_eq!(resolved.underline_color, Color([210, 210, 210]), "level {level}");
    }
}

#[test]
fn a_per_level_underline_override_only_changes_that_level() {
    let toml_text = r##"
        underline_width_pt = 2.0
        underline_color = "#d2d2d2"

        [levels.2]
        underline_width_pt = 0.0
    "##;
    let heading: HeadingStyle = toml::from_str(toml_text).unwrap();

    assert_eq!(heading.resolve(1).underline_width_pt, 2.0, "level 1 keeps the document-wide underline");
    assert_eq!(heading.resolve(2).underline_width_pt, 0.0, "level 2's own override turns its underline off");
    assert_eq!(heading.resolve(2).underline_color, Color([210, 210, 210]), "unset underline_color at level 2 still falls back to the document-wide value");
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
