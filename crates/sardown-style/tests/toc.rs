use sardown_style::{Stylesheet, TocStyle};

#[test]
fn default_toc_is_disabled_with_a_depth_of_two() {
    let toc = TocStyle::default();
    assert!(!toc.enabled);
    assert_eq!(toc.depth, 2);
    assert_eq!(toc.title, "Table of Contents");
    assert!(toc.page, "expected the in-document TOC page to be on by default when enabled");
}

#[test]
fn a_toml_document_can_enable_the_outline_without_an_in_document_toc_page() {
    let sheet: Stylesheet = toml::from_str("[toc]\nenabled = true\npage = false\n").unwrap();
    assert!(sheet.toc.enabled);
    assert!(!sheet.toc.page);
}

#[test]
fn a_toml_document_can_enable_the_toc_and_set_a_custom_depth() {
    let sheet: Stylesheet = toml::from_str("[toc]\nenabled = true\ndepth = 3\n").unwrap();
    assert!(sheet.toc.enabled);
    assert_eq!(sheet.toc.depth, 3);
}

#[test]
fn depth_zero_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-toc-depth-zero.toml");
    std::fs::write(&path, "[toc]\nenabled = true\ndepth = 0\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("depth"), "expected the error to mention depth, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn depth_seven_is_a_validation_error() {
    let path = std::env::temp_dir().join("sardown-test-toc-depth-seven.toml");
    std::fs::write(&path, "[toc]\nenabled = true\ndepth = 7\n").unwrap();
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("depth"), "expected the error to mention depth, got {err:?}");
    std::fs::remove_file(&path).unwrap();
}
