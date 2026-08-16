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
fn render_book_subcommand_produces_a_multi_page_pdf_from_an_mdbook_source_tree() {
    let out_path = std::env::temp_dir().join("md2pdf-test-book.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render-book").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mini-book")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).expect("output PDF was not written");
    assert!(bytes.starts_with(b"%PDF-"));
    let doc = lopdf::Document::load_mem(&bytes).expect("output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 2, "expected one page per chapter (each chapter forces its own page break)");
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

#[test]
fn render_book_loads_images_from_a_book_root_outside_the_current_directory() {
    // Regression test: render-book passed "." (the CLI process's current working directory) as
    // decode_images' security base_dir, instead of the book's own root. Every embedded image
    // path is already absolute by the time it reaches decode_images (md2pdf-book resolves each
    // chapter's images relative to that chapter's own directory), and decode_images' containment
    // check rejects any absolute path that isn't a descendant of base_dir -- so a book living
    // anywhere other than under the CLI's own CWD had every one of its images silently dropped
    // ("refusing to load image ...: path escapes base directory ...").
    let book_root = std::env::temp_dir().join("md2pdf-test-image-book");
    let _ = std::fs::remove_dir_all(&book_root);
    std::fs::create_dir_all(book_root.join("src")).unwrap();
    std::fs::write(book_root.join("src/SUMMARY.md"), "# Summary\n\n- [Intro](intro.md)\n").unwrap();
    std::fs::write(book_root.join("src/intro.md"), "# Intro\n\n![pic](test-image.png)\n").unwrap();
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/test-image.png"), book_root.join("src/test-image.png")).unwrap();

    let out_path = std::env::temp_dir().join("md2pdf-test-image-book.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render-book").arg(&book_root).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).expect("output PDF was not written");
    let doc = lopdf::Document::load_mem(&bytes).expect("output is not a valid PDF");
    let has_image_xobject =
        doc.objects.values().any(|obj| obj.as_stream().ok().and_then(|s| s.dict.get(b"Subtype").ok()).and_then(|s| s.as_name().ok()) == Some(b"Image"));
    assert!(has_image_xobject, "expected the chapter's image to be embedded in the output PDF");
}

#[test]
fn diagram_parse_error_reports_the_offending_line_inside_the_diagram_not_just_the_fence() {
    // Regression test: a failed-diagram warning previously only ever named the opening
    // ```mermaid fence's own location -- useless for finding the actual bad line in anything
    // but a one-line diagram. merman's parse errors carry a byte span pointing at the real
    // offending token; "bad syntax here" below is deliberately invalid on the diagram's own
    // 4th line (immediately after the 3-line fence + heading + blank line above it, so the
    // absolute file line is 3 + 4 = 7), and the warning should say so, not "line 3" (the fence).
    let md_path = std::env::temp_dir().join("md2pdf-test-diagram-location.md");
    std::fs::write(&md_path, "# Test\n\n```mermaid\nsequenceDiagram\n    participant A\n    participant B\n    A->>B bad syntax here\n    B-->>A: ok\n```\n")
        .unwrap();
    let out_path = std::env::temp_dir().join("md2pdf-test-diagram-location.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(&md_path).arg("-o").arg(&out_path);
    let output = cmd.output().expect("failed to run md2pdf");
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = format!("{}:7:11", md_path.display());
    assert!(stderr.contains(&expected), "expected stderr to contain {expected:?}, got:\n{stderr}");
}
