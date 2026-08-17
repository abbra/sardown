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
fn renders_successfully_after_the_font_loading_refactor() {
    let out_path = std::env::temp_dir().join("md2pdf-test-font-loading-regression.pdf");
    let _ = std::fs::remove_file(&out_path);

    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).expect("output PDF was not written");
    assert!(bytes.starts_with(b"%PDF-"));
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

#[test]
fn explicit_style_flag_changes_rendered_output() {
    // A drastically larger body font is a simple, unambiguous, easily-observed signal that the
    // stylesheet actually took effect through the full parse_with_style -> layout_with_header_footer
    // pipeline: the same source text overflows onto more pages at 60pt than at the default 12pt.
    // Uses large-book.md rather than basic.md -- basic.md is only one short sentence, too little
    // text to overflow onto extra pages at any body size (confirmed empirically: both renders
    // came out to exactly 1 page, so the two counts were never going to differ).
    let style_path = std::env::temp_dir().join("md2pdf-test-explicit-style.toml");
    std::fs::write(&style_path, "[typography]\nbody_size_pt = 60.0\n").unwrap();

    let styled_path = std::env::temp_dir().join("md2pdf-test-explicit-style-output.pdf");
    let _ = std::fs::remove_file(&styled_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/large-book.md")).arg("-o").arg(&styled_path).arg("--style").arg(&style_path);
    cmd.assert().success();
    let styled_doc = lopdf::Document::load_mem(&std::fs::read(&styled_path).unwrap()).unwrap();

    let default_path = std::env::temp_dir().join("md2pdf-test-explicit-style-default.pdf");
    let _ = std::fs::remove_file(&default_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/large-book.md")).arg("-o").arg(&default_path);
    cmd.assert().success();
    let default_doc = lopdf::Document::load_mem(&std::fs::read(&default_path).unwrap()).unwrap();

    assert!(styled_doc.get_pages().len() > default_doc.get_pages().len(), "expected the 60pt body style to overflow onto more pages than the default");

    std::fs::remove_file(&style_path).unwrap();
    std::fs::remove_file(&styled_path).unwrap();
    std::fs::remove_file(&default_path).unwrap();
}

#[test]
fn explicit_style_flag_exercises_code_block_label_rendering() {
    let style_path = std::env::temp_dir().join("md2pdf-test-code-label-style.toml");
    std::fs::write(&style_path, "[code_block]\nlabel_style = \"inline\"\n").unwrap();

    let out_path = std::env::temp_dir().join("md2pdf-test-code-label-output.pdf");
    let _ = std::fs::remove_file(&out_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md")).arg("-o").arg(&out_path).arg("--style").arg(&style_path);
    cmd.assert().success();

    let bytes = std::fs::read(&out_path).unwrap();
    let doc = lopdf::Document::load_mem(&bytes).unwrap();
    let text = doc.extract_text(&[1]).unwrap_or_default();
    assert!(text.contains("Rust"), "expected the auto-generated \"Rust\" code block label in the rendered output: {text}");

    std::fs::remove_file(&style_path).unwrap();
    std::fs::remove_file(&out_path).unwrap();
}

#[test]
fn render_book_auto_discovers_a_style_toml_in_the_book_root() {
    let book_root = std::env::temp_dir().join("md2pdf-test-style-auto-discovery-book");
    let _ = std::fs::remove_dir_all(&book_root);
    std::fs::create_dir_all(book_root.join("src")).unwrap();
    std::fs::write(book_root.join("src/SUMMARY.md"), "# Summary\n\n- [Intro](intro.md)\n").unwrap();
    // A single short sentence never overflows at any body size (confirmed empirically); repeat
    // one sentence enough times that inflating body_size_pt to 60pt visibly adds pages.
    let body = "This is a line of body text used to fill up space on the page. ".repeat(60);
    std::fs::write(book_root.join("src/intro.md"), format!("# Intro\n\n{body}\n")).unwrap();
    std::fs::write(book_root.join("style.toml"), "[typography]\nbody_size_pt = 60.0\n").unwrap();

    let styled_path = std::env::temp_dir().join("md2pdf-test-style-auto-discovery-output.pdf");
    let _ = std::fs::remove_file(&styled_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render-book").arg(&book_root).arg("-o").arg(&styled_path);
    cmd.assert().success();
    let styled_doc = lopdf::Document::load_mem(&std::fs::read(&styled_path).unwrap()).unwrap();

    // Same book, but with style.toml removed -- should fall back to defaults and fit on fewer
    // pages (a single short chapter easily fits on one page at the default 12pt body size).
    std::fs::remove_file(book_root.join("style.toml")).unwrap();
    let default_path = std::env::temp_dir().join("md2pdf-test-style-auto-discovery-default.pdf");
    let _ = std::fs::remove_file(&default_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render-book").arg(&book_root).arg("-o").arg(&default_path);
    cmd.assert().success();
    let default_doc = lopdf::Document::load_mem(&std::fs::read(&default_path).unwrap()).unwrap();

    assert!(
        styled_doc.get_pages().len() > default_doc.get_pages().len(),
        "expected the auto-discovered style.toml's 60pt body size to overflow onto more pages than the default"
    );

    std::fs::remove_dir_all(&book_root).unwrap();
    std::fs::remove_file(&styled_path).unwrap();
    std::fs::remove_file(&default_path).unwrap();
}

#[test]
fn explicit_style_flag_changes_the_embedded_font() {
    // Comparing the two PDFs' embedded /BaseFont names (rather than a raw byte-diff of the whole
    // file) specifically proves the *font* changed -- a byte-diff would also pass if the two
    // renders merely differed in some incidental non-deterministic metadata elsewhere in the
    // file, without actually proving font_family had any effect.
    // `Resources` is stored as an indirect reference, but `Font` (and the individual font
    // dictionaries within it) are stored inline -- confirmed by inspecting krilla's actual object
    // graph, which doesn't match the reference-everywhere assumption a naive `as_reference()`
    // walk would make. `Document::dereference` resolves either representation uniformly, so
    // every level below is looked up through it instead of assuming one or the other.
    fn base_fonts_of(doc: &lopdf::Document) -> std::collections::BTreeSet<Vec<u8>> {
        let mut names = std::collections::BTreeSet::new();
        for &page_id in doc.get_pages().values() {
            let Ok(page_dict) = doc.get_dictionary(page_id) else { continue };
            let Ok(resources_obj) = page_dict.get(b"Resources") else { continue };
            let Ok((_, resources_obj)) = doc.dereference(resources_obj) else { continue };
            let Ok(resources) = resources_obj.as_dict() else { continue };
            let Ok(fonts_obj) = resources.get(b"Font") else { continue };
            let Ok((_, fonts_obj)) = doc.dereference(fonts_obj) else { continue };
            let Ok(fonts) = fonts_obj.as_dict() else { continue };
            for (_, font_ref) in fonts.iter() {
                let Ok((_, font_obj)) = doc.dereference(font_ref) else { continue };
                let Ok(font_dict) = font_obj.as_dict() else { continue };
                if let Ok(base_font) = font_dict.get(b"BaseFont").and_then(|b| b.as_name()) {
                    names.insert(base_font.to_vec());
                }
            }
        }
        names
    }

    let style_path = std::env::temp_dir().join("md2pdf-test-font-family-style.toml");
    std::fs::write(&style_path, "[typography]\nfont_family = \"monospace\"\n").unwrap();

    let styled_path = std::env::temp_dir().join("md2pdf-test-font-family-output.pdf");
    let _ = std::fs::remove_file(&styled_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md")).arg("-o").arg(&styled_path).arg("--style").arg(&style_path);
    cmd.assert().success();
    let styled_doc = lopdf::Document::load_mem(&std::fs::read(&styled_path).unwrap()).unwrap();

    let default_path = std::env::temp_dir().join("md2pdf-test-font-family-default.pdf");
    let _ = std::fs::remove_file(&default_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md")).arg("-o").arg(&default_path);
    cmd.assert().success();
    let default_doc = lopdf::Document::load_mem(&std::fs::read(&default_path).unwrap()).unwrap();

    assert_ne!(
        base_fonts_of(&styled_doc),
        base_fonts_of(&default_doc),
        "expected a different embedded font when typography.font_family is set to \"monospace\""
    );

    std::fs::remove_file(&style_path).unwrap();
    std::fs::remove_file(&styled_path).unwrap();
    std::fs::remove_file(&default_path).unwrap();
}

#[test]
fn a_non_letter_page_format_produces_a_pdf_with_a_matching_physical_page_size() {
    // Regression test: md2pdf-pdf::render_pdf used to emit a hardcoded US-Letter-sized
    // /MediaBox on every page, regardless of what page format the stylesheet actually laid the
    // content out for. Content positioned relative to a taller/wider virtual page (e.g. A4, at
    // 842pt tall vs Letter's 792pt) than the physical page actually emitted got silently clipped
    // -- most visibly, a footer positioned near the bottom of an assumed-A4 page fell completely
    // outside the real, shorter Letter-sized page and never appeared, despite being present in
    // the PDF's own content stream.
    let out_path = std::env::temp_dir().join("md2pdf-test-a4-mediabox.pdf");
    let _ = std::fs::remove_file(&out_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md"))
        .arg("-o")
        .arg(&out_path)
        .arg("--style")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/style-examples/eu-a4.toml"));
    cmd.assert().success();

    let doc = lopdf::Document::load_mem(&std::fs::read(&out_path).unwrap()).unwrap();
    let &page_id = doc.get_pages().values().next().unwrap();
    let media_box = doc.get_dictionary(page_id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
    let width_pt = media_box[2].as_float().unwrap();
    let height_pt = media_box[3].as_float().unwrap();

    // A4 is 210mm x 297mm; 1mm == 72/25.4 pt.
    const PT_PER_MM: f32 = 72.0 / 25.4;
    let expected_width_pt = 210.0 * PT_PER_MM;
    let expected_height_pt = 297.0 * PT_PER_MM;
    assert!((width_pt - expected_width_pt).abs() < 1.0, "expected an A4-width MediaBox (~{expected_width_pt}pt), got {width_pt}pt");
    assert!((height_pt - expected_height_pt).abs() < 1.0, "expected an A4-height MediaBox (~{expected_height_pt}pt), got {height_pt}pt");

    std::fs::remove_file(&out_path).unwrap();
}

#[test]
fn explicit_style_flag_accepts_justified_alignment_without_disturbing_pagination() {
    // Precise visual proof that justification changes glyph positions lives in
    // md2pdf-layout's own unit tests (direct access to glyph x-coordinates). This test proves
    // the stylesheet field reaches the real end-to-end pipeline and that justifying body text
    // doesn't change how it wraps/paginates -- a real, meaningful, low-risk structural check.
    let style_path = std::env::temp_dir().join("md2pdf-test-alignment-style.toml");
    std::fs::write(&style_path, "[typography]\nalignment = \"justify\"\n").unwrap();

    let justified_path = std::env::temp_dir().join("md2pdf-test-alignment-justified.pdf");
    let _ = std::fs::remove_file(&justified_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/large-book.md")).arg("-o").arg(&justified_path).arg("--style").arg(&style_path);
    cmd.assert().success();
    let justified_doc = lopdf::Document::load_mem(&std::fs::read(&justified_path).unwrap()).unwrap();

    let default_path = std::env::temp_dir().join("md2pdf-test-alignment-default.pdf");
    let _ = std::fs::remove_file(&default_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/large-book.md")).arg("-o").arg(&default_path);
    cmd.assert().success();
    let default_doc = lopdf::Document::load_mem(&std::fs::read(&default_path).unwrap()).unwrap();

    assert_eq!(justified_doc.get_pages().len(), default_doc.get_pages().len(), "expected justification to change glyph spacing, not the number of pages");

    std::fs::remove_file(&style_path).unwrap();
    std::fs::remove_file(&justified_path).unwrap();
    std::fs::remove_file(&default_path).unwrap();
}

#[test]
fn explicit_style_flag_generates_a_toc_page_with_working_links() {
    let style_path = std::env::temp_dir().join("md2pdf-test-toc-style.toml");
    std::fs::write(&style_path, "[toc]\nenabled = true\ndepth = 2\n").unwrap();

    let md_path = std::env::temp_dir().join("md2pdf-test-toc-doc.md");
    std::fs::write(&md_path, "# Chapter One\n\nBody.\n\n## Section A\n\nMore body.\n").unwrap();

    let out_path = std::env::temp_dir().join("md2pdf-test-toc-output.pdf");
    let _ = std::fs::remove_file(&out_path);
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(&md_path).arg("-o").arg(&out_path).arg("--style").arg(&style_path);
    cmd.assert().success();

    let doc = lopdf::Document::load_mem(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(doc.get_pages().len(), 2, "expected 1 TOC page + 1 content page");
    let toc_text = doc.extract_text(&[1]).unwrap_or_default();
    assert!(toc_text.contains("Table of Contents"), "expected the TOC title on page 1: {toc_text}");
    assert!(toc_text.contains("Chapter One"), "expected a Chapter One entry on page 1: {toc_text}");
    assert!(toc_text.contains("Section A"), "expected a Section A entry on page 1: {toc_text}");
    let has_outlines = doc.catalog().ok().and_then(|cat| cat.get(b"Outlines").ok()).is_some();
    assert!(has_outlines, "expected a populated PDF outline");

    std::fs::remove_file(&style_path).unwrap();
    std::fs::remove_file(&md_path).unwrap();
    std::fs::remove_file(&out_path).unwrap();
}
