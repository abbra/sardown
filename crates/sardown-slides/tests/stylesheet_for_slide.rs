use sardown_slides::{apply_slide_scale, build_slide_stylesheet};
use sardown_style::{SlideLayoutStyle, Stylesheet, TextAlignment};

#[test]
fn scale_1_0_with_no_layout_overrides_leaves_font_sizes_unchanged() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle::default();
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    assert_eq!(sheet.typography.body_size_pt, base.typography.body_size_pt);
    assert_eq!(sheet.table.text_size_pt, base.table.text_size_pt);
    assert_eq!(sheet.code_block.default.font_size_pt, base.code_block.default.font_size_pt);
}

#[test]
fn scale_0_5_halves_every_content_font_size_but_not_header_footer() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle::default();
    let sheet = build_slide_stylesheet(&base, &layout, 0.5);
    assert_eq!(sheet.typography.body_size_pt, base.typography.body_size_pt * 0.5);
    assert_eq!(sheet.table.text_size_pt, base.table.text_size_pt * 0.5);
    assert_eq!(sheet.code_block.default.font_size_pt, base.code_block.default.font_size_pt * 0.5);
    assert_eq!(sheet.header.font_size_pt, base.header.font_size_pt, "header chrome must never be scaled");
    assert_eq!(sheet.footer.font_size_pt, base.footer.font_size_pt, "footer chrome must never be scaled");
}

#[test]
fn a_layouts_body_size_override_is_the_unscaled_base_before_scaling() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle { body_size_pt: Some(20.0), ..SlideLayoutStyle::default() };
    let sheet = build_slide_stylesheet(&base, &layout, 0.5);
    assert_eq!(sheet.typography.body_size_pt, 10.0, "20.0 (layout override) * 0.5 (scale)");
}

#[test]
fn a_layouts_alignment_override_applies() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle { alignment: Some(TextAlignment::Center), ..SlideLayoutStyle::default() };
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    assert_eq!(sheet.typography.alignment, TextAlignment::Center);
}

#[test]
fn a_layouts_text_color_override_applies_to_body_color() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle { text_color: Some(sardown_style::Color([255, 255, 255])), ..SlideLayoutStyle::default() };
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    assert_eq!(sheet.typography.body_color, sardown_style::Color([255, 255, 255]));
}

#[test]
fn a_layouts_text_color_override_applies_to_every_heading_levels_underline_color() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle { text_color: Some(sardown_style::Color([26, 74, 122])), ..SlideLayoutStyle::default() };
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    for level in 1..=6u8 {
        assert_eq!(
            sheet.heading.resolve(level).underline_color,
            sardown_style::Color([26, 74, 122]),
            "level {level} underline color should follow the layout's text_color"
        );
    }
}

#[test]
fn a_layouts_text_color_override_replaces_a_pre_existing_per_level_underline_color() {
    let toml_text = "[heading.levels.1]\nunderline_color = \"#d2d2d2\"\n";
    let base: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = SlideLayoutStyle { text_color: Some(sardown_style::Color([26, 74, 122])), ..SlideLayoutStyle::default() };
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    assert_eq!(sheet.heading.resolve(1).underline_color, sardown_style::Color([26, 74, 122]));
}

#[test]
fn no_layout_text_color_leaves_heading_underline_colors_untouched() {
    let toml_text = "[heading.levels.1]\nunderline_color = \"#d2d2d2\"\n";
    let base: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = SlideLayoutStyle::default();
    let sheet = build_slide_stylesheet(&base, &layout, 1.0);
    assert_eq!(sheet.heading.resolve(1).underline_color, sardown_style::Color([210, 210, 210]));
}

#[test]
fn scaling_a_per_language_code_block_font_size_override_leaves_unset_languages_alone() {
    let toml_text = "[code_block.languages.rust]\nfont_size_pt = 14.0\n\n[code_block.languages.python]\nlabel = \"Python\"\n";
    let base: Stylesheet = toml::from_str(toml_text).unwrap();
    let layout = SlideLayoutStyle::default();
    let sheet = build_slide_stylesheet(&base, &layout, 0.5);
    assert_eq!(sheet.code_block.languages.get("rust").unwrap().font_size_pt, Some(7.0));
    assert_eq!(sheet.code_block.languages.get("python").unwrap().font_size_pt, None);
}

#[test]
fn in_place_scaling_matches_a_fresh_build_at_every_scale() {
    let base: Stylesheet = toml::from_str("[code_block.languages.rust]\nfont_size_pt = 14.0\n\n[typography]\nbody_size_pt = 20.0\n").unwrap();
    let layout = SlideLayoutStyle { body_size_pt: Some(24.0), ..SlideLayoutStyle::default() };
    // One-shot reference values.
    let fresh = build_slide_stylesheet(&base, &layout, 0.7);
    // Incremental path: override step once at scale 1.0, then scale in place -- the shrink
    // loop's exact sequence.
    let mut incremental = build_slide_stylesheet(&base, &layout, 1.0);
    apply_slide_scale(&mut incremental, &base, &layout, 0.7);
    assert_eq!(incremental.typography.body_size_pt, fresh.typography.body_size_pt, "24.0 (override) * 0.7");
    assert_eq!(incremental.table.text_size_pt, fresh.table.text_size_pt);
    assert_eq!(incremental.code_block.default.font_size_pt, fresh.code_block.default.font_size_pt);
    assert_eq!(
        incremental.code_block.languages.get("rust").unwrap().font_size_pt,
        fresh.code_block.languages.get("rust").unwrap().font_size_pt,
        "per-language overrides must scale identically in both paths"
    );
}

#[test]
fn repeated_in_place_scaling_never_compounds() {
    let base = Stylesheet::default();
    let layout = SlideLayoutStyle::default();
    let mut sheet = build_slide_stylesheet(&base, &layout, 1.0);
    for scale in [0.95f32, 0.9, 0.85] {
        apply_slide_scale(&mut sheet, &base, &layout, scale);
        assert_eq!(sheet.typography.body_size_pt, base.typography.body_size_pt * scale);
        assert_eq!(sheet.code_block.default.font_size_pt, base.code_block.default.font_size_pt * scale);
    }
}
