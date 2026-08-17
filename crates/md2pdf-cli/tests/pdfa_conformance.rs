use assert_cmd::Command;

/// Independently verifies md2pdf's own PDF/A-2b conformance claim (stated throughout this
/// project's documentation) against veraPDF, the validator the original architecture design
/// itself specified for this purpose -- krilla's own internal enforcement (e.g. refusing to
/// serialize a `.notdef` glyph) covers only what krilla's authors chose to check, not full
/// PDF/A-2b conformance. Gated on `VERAPDF_BIN` (a path to a real `verapdf` executable) the same
/// way `visual_regression.rs`'s tests are gated on `PDFIUM_DYNAMIC_LIB_PATH`: this test fails
/// loudly with a clear message if the variable isn't set, rather than silently skipping, so a
/// missing environment is never mistaken for a passing conformance check.
///
/// All fixtures are validated in one `verapdf` invocation (not one process per fixture) since
/// each invocation pays real JVM startup cost.
fn verapdf_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("VERAPDF_BIN").expect("VERAPDF_BIN not set -- point it at a real verapdf executable"))
}

/// Extracts `(file_name, is_compliant)` for every `<job>` in veraPDF's XML report, by simple
/// string scanning rather than a full XML parser -- the report's per-job structure is small and
/// predictable enough (one `<name>` and one `isCompliant="..."` per job, in that order) that a
/// real parser dependency isn't justified just for this.
fn parse_compliance(report_xml: &str) -> Vec<(String, bool)> {
    let mut results = Vec::new();
    for job in report_xml.split("<job>").skip(1) {
        let name = job
            .split_once("<name>")
            .and_then(|(_, rest)| rest.split_once("</name>"))
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| "(unknown)".to_string());
        let compliant = job.contains("isCompliant=\"true\"");
        results.push((name, compliant));
    }
    results
}

#[test]
fn rendered_output_is_pdf_a_2b_compliant() {
    let verapdf = verapdf_bin();
    let tmp = std::env::temp_dir();

    let mut fixtures = Vec::new();

    let basic_pdf = tmp.join("md2pdf-test-pdfa-basic.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md"), "-o"])
        .arg(&basic_pdf)
        .assert()
        .success();
    fixtures.push(basic_pdf);

    // Exercises syntax-highlighted code blocks, tables, and formatted inline text.
    let formatting_pdf = tmp.join("md2pdf-test-pdfa-formatting.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md"), "-o"])
        .arg(&formatting_pdf)
        .assert()
        .success();
    fixtures.push(formatting_pdf);

    // Exercises Mermaid-diagram vector content and /Link annotations.
    let linking_pdf = tmp.join("md2pdf-test-pdfa-linking.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linking.md"), "-o"])
        .arg(&linking_pdf)
        .assert()
        .success();
    fixtures.push(linking_pdf);

    // Exercises a multi-chapter render-book output with cross-chapter links.
    let book_pdf = tmp.join("md2pdf-test-pdfa-book.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render-book", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mini-book"), "-o"])
        .arg(&book_pdf)
        .assert()
        .success();
    fixtures.push(book_pdf);

    // Exercises an embedded raster image.
    let image_dir = tmp.join("md2pdf-test-pdfa-image-src");
    let _ = std::fs::remove_dir_all(&image_dir);
    std::fs::create_dir_all(&image_dir).unwrap();
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/test-image.png"), image_dir.join("test-image.png")).unwrap();
    std::fs::write(image_dir.join("doc.md"), "# Image Test\n\n![test](test-image.png)\n").unwrap();
    let image_pdf = tmp.join("md2pdf-test-pdfa-image.pdf");
    Command::cargo_bin("md2pdf").unwrap().args(["render"]).arg(image_dir.join("doc.md")).arg("-o").arg(&image_pdf).assert().success();
    fixtures.push(image_pdf.clone());

    // Exercises a custom embedded font loaded via typography.font_dirs.
    let font_style_path = tmp.join("md2pdf-test-pdfa-font-style.toml");
    let font_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures");
    std::fs::write(&font_style_path, format!("[typography]\nfont_family = \"Droid Sans\"\nuse_system_fonts = false\nfont_dirs = [\"{font_dir}\"]\n")).unwrap();
    let font_pdf = tmp.join("md2pdf-test-pdfa-font.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/basic.md"), "-o"])
        .arg(&font_pdf)
        .arg("--style")
        .arg(&font_style_path)
        .assert()
        .success();
    fixtures.push(font_pdf.clone());

    // Exercises an embedded arbitrary (non-Mermaid-generated) .svg image -- unlike the other
    // vector content this project already validated, this SVG source didn't come from merman.
    let svg_dir = tmp.join("md2pdf-test-pdfa-svg-src");
    let _ = std::fs::remove_dir_all(&svg_dir);
    std::fs::create_dir_all(&svg_dir).unwrap();
    std::fs::write(
        svg_dir.join("test-vector.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\" viewBox=\"0 0 100 50\">\n  <rect width=\"100\" height=\"50\" fill=\"#3366cc\"/>\n</svg>\n",
    )
    .unwrap();
    std::fs::write(svg_dir.join("doc.md"), "# SVG Image Test\n\n![test](test-vector.svg)\n").unwrap();
    let svg_pdf = tmp.join("md2pdf-test-pdfa-svg.pdf");
    Command::cargo_bin("md2pdf").unwrap().args(["render"]).arg(svg_dir.join("doc.md")).arg("-o").arg(&svg_pdf).assert().success();
    fixtures.push(svg_pdf.clone());

    // Exercises hyphenated output -- a literal hyphen character and forced line break the
    // validator has never seen from this project before. basic.md's own short words never need
    // to hyphenate, so this uses a dedicated fixture with a genuinely long word instead.
    let hyphenation_style_path = tmp.join("md2pdf-test-pdfa-hyphenation-style.toml");
    std::fs::write(&hyphenation_style_path, "[page]\nwidth_mm = 60.0\nheight_mm = 279.4\nmargin_mm = 5.0\n\n[typography]\nhyphenation = true\n").unwrap();
    let hyphenation_doc_path = tmp.join("md2pdf-test-pdfa-hyphenation-doc.md");
    std::fs::write(&hyphenation_doc_path, "An extraordinarily long hyphenation demonstration paragraph that must wrap across several lines.\n").unwrap();
    let hyphenation_pdf = tmp.join("md2pdf-test-pdfa-hyphenation.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render"])
        .arg(&hyphenation_doc_path)
        .arg("-o")
        .arg(&hyphenation_pdf)
        .arg("--style")
        .arg(&hyphenation_style_path)
        .assert()
        .success();
    fixtures.push(hyphenation_pdf.clone());

    // Exercises the render-slides subcommand -- background fills and vertical-centering shifts
    // are new content shapes this validator hasn't seen from md2pdf before.
    let slides_pdf = tmp.join("md2pdf-test-pdfa-slides.pdf");
    Command::cargo_bin("md2pdf")
        .unwrap()
        .args(["render-slides", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/slides-deck.md"), "-o"])
        .arg(&slides_pdf)
        .arg("--style")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/slides-style.toml"))
        .assert()
        .success();
    fixtures.push(slides_pdf.clone());

    let output = std::process::Command::new(&verapdf)
        .arg("--flavour")
        .arg("2b")
        .args(&fixtures)
        .output()
        .unwrap_or_else(|e| panic!("failed to run verapdf at {}: {e}", verapdf.display()));
    let report = String::from_utf8_lossy(&output.stdout);
    let results = parse_compliance(&report);

    assert_eq!(results.len(), fixtures.len(), "expected one report entry per rendered fixture, got:\n{report}");
    let non_compliant: Vec<_> = results.iter().filter(|(_, compliant)| !compliant).collect();
    assert!(non_compliant.is_empty(), "expected every fixture to be PDF/A-2b compliant, but these weren't: {non_compliant:?}\nfull report:\n{report}");

    for pdf in &fixtures {
        let _ = std::fs::remove_file(pdf);
    }
    let _ = std::fs::remove_file(&font_style_path);
    let _ = std::fs::remove_dir_all(&image_dir);
    let _ = std::fs::remove_dir_all(&svg_dir);
    let _ = std::fs::remove_file(&hyphenation_style_path);
    let _ = std::fs::remove_file(&hyphenation_doc_path);
}
