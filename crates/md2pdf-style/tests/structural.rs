use md2pdf_style::{BlockquoteStyle, Color, ListStyle, ThematicBreakStyle};

#[test]
fn default_blockquote_style_matches_todays_hardcoded_border() {
    let blockquote = BlockquoteStyle::default();
    assert_eq!(blockquote.border_color, Color([180, 180, 180]));
    assert_eq!(blockquote.border_width_pt, 2.0);
    assert_eq!(blockquote.indent_pt, 18.0);
}

#[test]
fn default_thematic_break_style_matches_todays_hardcoded_rule() {
    let thematic_break = ThematicBreakStyle::default();
    assert_eq!(thematic_break.color, Color([200, 200, 200]));
    assert_eq!(thematic_break.width_pt, 1.0);
}

#[test]
fn default_list_style_matches_todays_hardcoded_indent() {
    assert_eq!(ListStyle::default().indent_pt, 18.0);
}

#[test]
fn a_partial_blockquote_override_keeps_the_other_fields_default() {
    let blockquote: BlockquoteStyle = toml::from_str("border_width_pt = 3.0").unwrap();
    assert_eq!(blockquote.border_width_pt, 3.0);
    assert_eq!(blockquote.border_color, Color([180, 180, 180]));
    assert_eq!(blockquote.indent_pt, 18.0);
}
