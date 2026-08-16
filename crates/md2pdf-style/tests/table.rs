use md2pdf_style::TableStyle;

#[test]
fn default_table_style_matches_todays_hardcoded_padding_and_text_size() {
    let table = TableStyle::default();
    assert_eq!(table.cell_padding_pt, 12.0);
    assert_eq!(table.text_size_pt, 10.5);
    assert_eq!(table.min_row_height_pt, 20.0);
}

#[test]
fn a_partial_override_keeps_the_other_fields_default() {
    let table: TableStyle = toml::from_str("text_size_pt = 9.0").unwrap();
    assert_eq!(table.text_size_pt, 9.0);
    assert_eq!(table.cell_padding_pt, 12.0);
    assert_eq!(table.min_row_height_pt, 20.0);
}
