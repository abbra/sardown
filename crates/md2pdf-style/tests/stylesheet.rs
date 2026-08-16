use md2pdf_style::{Color, LabelStyle, PageFormat, Stylesheet};

#[test]
fn default_stylesheet_matches_every_currently_hardcoded_value() {
    let sheet = Stylesheet::default();

    assert_eq!(sheet.page.format, PageFormat::Letter);
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 279.4));
    assert_eq!(sheet.page.margin_mm, 25.4);

    assert_eq!(sheet.typography.font_family, "sans-serif");
    assert_eq!(sheet.typography.body_size_pt, 12.0);
    assert_eq!(sheet.typography.body_color, Color([0, 0, 0]));
    assert!(sheet.typography.use_system_fonts);
    assert!(sheet.typography.font_dirs.is_empty());

    assert_eq!(sheet.heading.space_before_factor, 0.8);
    for (level, expected_size) in [(1u8, 28.0), (2, 22.0), (3, 18.0), (4, 16.0), (5, 14.0), (6, 12.0)] {
        let resolved = sheet.heading.resolve(level);
        assert_eq!(resolved.size_pt, expected_size, "level {level}");
        assert_eq!(resolved.color, Color([0, 0, 0]), "level {level}");
        assert_eq!(resolved.font_family, "sans-serif", "level {level}");
    }

    assert_eq!(sheet.blockquote.border_color, Color([180, 180, 180]));
    assert_eq!(sheet.blockquote.border_width_pt, 2.0);
    assert_eq!(sheet.blockquote.indent_pt, 18.0);

    assert_eq!(sheet.thematic_break.color, Color([200, 200, 200]));
    assert_eq!(sheet.thematic_break.width_pt, 1.0);

    assert_eq!(sheet.list.indent_pt, 18.0);

    assert_eq!(sheet.table.cell_padding_pt, 12.0);
    assert_eq!(sheet.table.text_size_pt, 10.5);
    assert_eq!(sheet.table.min_row_height_pt, 20.0);

    assert_eq!(sheet.code_block.syntax_theme, "InspiredGitHub");
    assert_eq!(sheet.code_block.label_style, LabelStyle::None);
    assert_eq!(sheet.code_block.default_label, "text");
    assert_eq!(sheet.code_block.default.background, Color([245, 245, 245]));
    assert_eq!(sheet.code_block.default.font_family, "monospace");
    assert_eq!(sheet.code_block.default.font_size_pt, 10.0);
}

#[test]
fn an_empty_toml_document_deserializes_to_the_full_default_stylesheet() {
    let sheet: Stylesheet = toml::from_str("").unwrap();
    let default_sheet = Stylesheet::default();
    assert_eq!(sheet.page.dimensions_mm(), default_sheet.page.dimensions_mm());
    assert_eq!(sheet.table.text_size_pt, default_sheet.table.text_size_pt);
}

#[test]
fn a_toml_document_touching_one_section_leaves_every_other_section_default() {
    let toml_text = r#"
        [table]
        text_size_pt = 9.0
    "#;
    let sheet: Stylesheet = toml::from_str(toml_text).unwrap();
    assert_eq!(sheet.table.text_size_pt, 9.0);
    assert_eq!(sheet.page.format, PageFormat::Letter);
    assert_eq!(sheet.typography.body_size_pt, 12.0);
    assert_eq!(sheet.code_block.syntax_theme, "InspiredGitHub");
}

#[test]
fn stylesheet_default_has_header_and_footer_disabled() {
    let sheet = Stylesheet::default();
    assert!(!sheet.header.enabled);
    assert!(!sheet.footer.enabled);
}
