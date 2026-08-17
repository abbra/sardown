use md2pdf_style::DocumentStyle;

#[test]
fn default_document_style_has_empty_title_author_and_date() {
    let doc = DocumentStyle::default();
    assert_eq!(doc.title, "");
    assert_eq!(doc.author, "");
    assert_eq!(doc.date, "");
}

#[test]
fn deserializes_title_and_author_from_a_partial_toml_table() {
    let doc: DocumentStyle = toml::from_str(r#"title = "My Book""#).unwrap();
    assert_eq!(doc.title, "My Book");
    assert_eq!(doc.author, "");
}

#[test]
fn deserializes_a_static_date_from_toml() {
    let doc: DocumentStyle = toml::from_str(r#"date = "2026-01-01""#).unwrap();
    assert_eq!(doc.date, "2026-01-01");
}
