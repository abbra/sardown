use md2pdf_style::{Color, PageFormat, Stylesheet};
use std::io::Write;

fn write_temp_toml(contents: &str) -> std::path::PathBuf {
    // Tests in this file run concurrently as threads within one process, and some tests write
    // more than one temp file -- keying solely on process id (shared by every call here) caused
    // concurrent tests to clobber and delete each other's files mid-run. The counter guarantees a
    // distinct path per call.
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("md2pdf-style-test-{}-{n}.toml", std::process::id()));
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    path
}

#[test]
fn loads_a_toml_file_that_only_sets_one_field_and_keeps_every_other_default() {
    let path = write_temp_toml("[table]\ntext_size_pt = 9.0\n");
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.table.text_size_pt, 9.0);
    assert_eq!(sheet.page.format, PageFormat::Letter);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn loads_hex_and_rgb_array_colors_the_same_way() {
    let path_hex = write_temp_toml("[blockquote]\nborder_color = \"#ff0000\"\n");
    let path_rgb = write_temp_toml("[blockquote]\nborder_color = [255, 0, 0]\n");
    let sheet_hex = Stylesheet::load(&path_hex).unwrap();
    let sheet_rgb = Stylesheet::load(&path_rgb).unwrap();
    assert_eq!(sheet_hex.blockquote.border_color, Color([255, 0, 0]));
    assert_eq!(sheet_rgb.blockquote.border_color, Color([255, 0, 0]));
    std::fs::remove_file(&path_hex).unwrap();
    std::fs::remove_file(&path_rgb).unwrap();
}

#[test]
fn loads_a_named_page_format() {
    let path = write_temp_toml("[page]\nformat = \"a4\"\n");
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (210.0, 297.0));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn rejects_a_page_size_with_only_width_mm_set() {
    let path = write_temp_toml("[page]\nwidth_mm = 200.0\n");
    let err = Stylesheet::load(&path).unwrap_err();
    // anyhow::Error's Display (`{err}`) shows only the outermost .with_context() message; the
    // full cause chain (where validate()'s specific message lives) needs Debug (`{err:?}`).
    assert!(format!("{err:?}").contains("height_mm"), "expected error to name the missing field, got: {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn rejects_a_page_size_with_only_height_mm_set() {
    let path = write_temp_toml("[page]\nheight_mm = 200.0\n");
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("width_mm"), "expected error to name the missing field, got: {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn rejects_a_page_style_with_only_inner_margin_mm_set() {
    let path = write_temp_toml("[page]\ninner_margin_mm = 30.0\n");
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("outer_margin_mm"), "expected error to name the missing field, got: {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn rejects_a_page_style_with_only_outer_margin_mm_set() {
    let path = write_temp_toml("[page]\nouter_margin_mm = 20.0\n");
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("inner_margin_mm"), "expected error to name the missing field, got: {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn accepts_a_page_style_with_both_inner_and_outer_margin_mm_set() {
    let path = write_temp_toml("[page]\ninner_margin_mm = 30.0\nouter_margin_mm = 20.0\n");
    let sheet = Stylesheet::load(&path).unwrap();
    assert_eq!(sheet.page.inner_margin_mm, Some(30.0));
    assert_eq!(sheet.page.outer_margin_mm, Some(20.0));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn rejects_an_invalid_hex_color() {
    let path = write_temp_toml("[blockquote]\nborder_color = \"not-a-color\"\n");
    assert!(Stylesheet::load(&path).is_err());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn loading_a_nonexistent_file_is_an_error() {
    let path = std::env::temp_dir().join("md2pdf-style-test-does-not-exist.toml");
    let _ = std::fs::remove_file(&path);
    assert!(Stylesheet::load(&path).is_err());
}

#[test]
fn rejects_an_unknown_header_placeholder() {
    let path = write_temp_toml("[header]\nenabled = true\n[header.uniform]\nleft = \"{bogus}\"\n");
    let err = Stylesheet::load(&path).unwrap_err();
    assert!(format!("{err:?}").contains("bogus"), "expected the bad placeholder name in the error, got: {err:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn accepts_a_valid_header_and_footer_configuration() {
    let toml_text = "[header]\nenabled = true\n[header.uniform]\ncenter = \"{h1}\"\n\n[footer]\nenabled = true\n[footer.uniform]\ncenter = \"Page {page} of {total_pages}\"\n";
    let path = write_temp_toml(toml_text);
    let sheet = Stylesheet::load(&path).unwrap();
    assert!(sheet.header.enabled);
    assert_eq!(sheet.header.uniform.center, "{h1}");
    assert!(sheet.footer.enabled);
    std::fs::remove_file(&path).unwrap();
}
