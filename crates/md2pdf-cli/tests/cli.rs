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

#[test]
fn renders_all_phase_2_block_kinds_to_a_valid_pdf() {
    let out_path = std::env::temp_dir().join("md2pdf-test-formatting.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).unwrap();
    let doc = lopdf::Document::load_mem(&bytes).expect("output is not a valid PDF");
    assert!(!doc.get_pages().is_empty());

    let text = doc.extract_text(&[1]).unwrap_or_default();
    assert!(text.contains("Formatting Test"), "heading text missing: {text}");
    assert!(text.contains("Col A"), "table header text missing: {text}");
    // Each syntax-highlighted token is its own separate Tj call (a real PDF requirement, since
    // color is graphics state applied per text-showing operation), and lopdf's extract_text
    // doesn't reconstruct continuous words/phrases across separate same-line calls with
    // different colors -- confirmed by direct content-stream inspection that "fn"/"main" render
    // as correctly-flowing, correctly-grouped-by-line glyphs. So check the tokens individually
    // rather than asserting the exact contiguous phrase "fn main" survives extraction.
    assert!(text.contains("fn"), "code block text missing 'fn': {text}");
    assert!(text.contains("main"), "code block text missing 'main': {text}");
}

#[test]
fn renders_diagrams_and_links_to_a_valid_pdf() {
    let out_path = std::env::temp_dir().join("md2pdf-test-linking.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linking.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).unwrap();
    let doc = lopdf::Document::load_mem(&bytes).expect("output is not a valid PDF");
    assert!(!doc.get_pages().is_empty());

    // Confirm at least one /Link annotation made it into the page tree.
    let has_link_annot = doc.get_pages().values().any(|&page_id| doc.get_dictionary(page_id).and_then(|d| d.get(b"Annots")).is_ok());
    assert!(has_link_annot, "expected at least one /Annots entry in the output PDF");
}
