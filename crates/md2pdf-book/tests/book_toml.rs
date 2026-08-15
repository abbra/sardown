fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).join(name)
}

#[test]
fn reads_custom_src_dir_from_book_toml() {
    let src = md2pdf_book::resolve_src_dir(&fixture("with-custom-src"));
    assert_eq!(src, fixture("with-custom-src").join("pages"));
}

#[test]
fn defaults_to_src_when_book_toml_has_no_src_key() {
    let src = md2pdf_book::resolve_src_dir(&fixture("with-default-src"));
    assert_eq!(src, fixture("with-default-src").join("src"));
}

#[test]
fn defaults_to_src_when_book_toml_is_missing_entirely() {
    let src = md2pdf_book::resolve_src_dir(&fixture("no-book-toml"));
    assert_eq!(src, fixture("no-book-toml").join("src"));
}
