use assert_cmd::Command;

#[test]
fn render_subcommand_requires_input_and_output() {
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg("nonexistent.md").arg("-o").arg("/tmp/md2pdf-test-missing.pdf");
    cmd.assert().failure();
}

#[test]
fn renders_basic_markdown_to_a_valid_single_page_pdf() {
    let out_path = std::env::temp_dir().join("md2pdf-test-basic.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).expect("output PDF was not written");
    assert!(bytes.starts_with(b"%PDF-"));
    let doc = lopdf::Document::load_mem(&bytes).expect("output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 1);
}
