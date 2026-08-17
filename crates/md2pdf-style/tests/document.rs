use md2pdf_style::DocumentStyle;

#[test]
fn default_document_style_has_empty_title_and_author() {
    let doc = DocumentStyle::default();
    assert_eq!(doc.title, "");
    assert_eq!(doc.author, "");
}

#[test]
fn deserializes_title_and_author_from_a_partial_toml_table() {
    let doc: DocumentStyle = toml::from_str(r#"title = "My Book""#).unwrap();
    assert_eq!(doc.title, "My Book");
    assert_eq!(doc.author, "");
}
