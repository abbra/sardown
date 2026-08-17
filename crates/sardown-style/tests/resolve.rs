use sardown_style::Stylesheet;
use std::io::Write;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sardown-style-resolve-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_toml(path: &std::path::Path, contents: &str) {
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
}

#[test]
fn resolve_with_neither_argument_returns_the_default_stylesheet() {
    let sheet = Stylesheet::resolve(None, None).unwrap();
    assert_eq!(sheet.table.text_size_pt, Stylesheet::default().table.text_size_pt);
}

#[test]
fn resolve_falls_back_to_book_root_style_toml_when_present() {
    let dir = temp_dir("book-root-hit");
    write_toml(&dir.join("style.toml"), "[table]\ntext_size_pt = 9.0\n");
    let sheet = Stylesheet::resolve(None, Some(&dir)).unwrap();
    assert_eq!(sheet.table.text_size_pt, 9.0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolve_falls_back_to_default_when_book_root_has_no_style_toml() {
    let dir = temp_dir("book-root-miss");
    let sheet = Stylesheet::resolve(None, Some(&dir)).unwrap();
    assert_eq!(sheet.table.text_size_pt, Stylesheet::default().table.text_size_pt);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolve_prefers_an_explicit_path_over_book_root_auto_discovery() {
    let dir = temp_dir("explicit-wins");
    write_toml(&dir.join("style.toml"), "[table]\ntext_size_pt = 9.0\n");
    let explicit_path = dir.join("explicit.toml");
    write_toml(&explicit_path, "[table]\ntext_size_pt = 7.0\n");

    let sheet = Stylesheet::resolve(Some(&explicit_path), Some(&dir)).unwrap();
    assert_eq!(sheet.table.text_size_pt, 7.0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolve_propagates_a_parse_error_from_an_explicit_path() {
    let dir = temp_dir("explicit-invalid");
    let explicit_path = dir.join("bad.toml");
    write_toml(&explicit_path, "[page]\nwidth_mm = 200.0\n");

    assert!(Stylesheet::resolve(Some(&explicit_path), None).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
