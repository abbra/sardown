use assert_cmd::Command;
use pdfium_render::prelude::*;

const PIXEL_DIFF_THRESHOLD: f64 = 0.02; // fraction of differing pixels tolerated (anti-aliasing jitter)

fn render_first_page_to_rgba(pdf_path: &std::path::Path) -> image::RgbaImage {
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
            &std::env::var("PDFIUM_DYNAMIC_LIB_PATH").expect("PDFIUM_DYNAMIC_LIB_PATH not set"),
        ))
        .expect("failed to bind to libpdfium — check PDFIUM_DYNAMIC_LIB_PATH"),
    );
    let document = pdfium.load_pdf_from_file(pdf_path, None).expect("failed to load rendered PDF");
    let page = document.pages().get(0).expect("PDF has no pages");
    let bitmap = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(800))
        .expect("failed to rasterize page");
    bitmap.as_image().expect("failed to convert pdfium bitmap to image").to_rgba8()
}

fn fraction_differing(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "golden and candidate images have different dimensions");
    let total = a.pixels().len() as f64;
    let differing = a.pixels().zip(b.pixels()).filter(|(p1, p2)| p1 != p2).count() as f64;
    differing / total
}

#[test]
fn formatting_fixture_matches_golden_render() {
    let out_path = std::env::temp_dir().join("md2pdf-visual-formatting.pdf");
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let candidate = render_first_page_to_rgba(&out_path);
    let golden_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/formatting-page-1.png");
    let golden =
        image::open(golden_path).unwrap_or_else(|_| panic!("no golden image at {golden_path} — run with --ignored update_golden first, see Step 4")).to_rgba8();

    let diff = fraction_differing(&golden, &candidate);
    assert!(diff < PIXEL_DIFF_THRESHOLD, "rendered output diverged from golden by {:.2}% of pixels", diff * 100.0);
}

#[test]
#[ignore] // run explicitly: cargo test -p md2pdf-cli --test visual_regression -- --ignored update_golden
fn update_golden() {
    let out_path = std::env::temp_dir().join("md2pdf-visual-formatting.pdf");
    let mut cmd = Command::cargo_bin("md2pdf").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md")).arg("-o").arg(&out_path);
    cmd.assert().success();
    let candidate = render_first_page_to_rgba(&out_path);
    candidate.save(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/formatting-page-1.png")).expect("failed to save golden image");
}
