use assert_cmd::Command;
use pdfium_render::prelude::*;
use std::sync::{Mutex, OnceLock};

const PIXEL_DIFF_THRESHOLD: f64 = 0.02; // fraction of differing pixels tolerated (anti-aliasing jitter)

// `Pdfium::new` registers its bindings in a process-global `OnceCell` and panics if that's
// already set -- now that this file has more than one test that rasterizes a PDF, Rust's default
// parallel test execution meant the second test to run hit "PdfiumLibraryBindingsAlreadyInitialized"
// (or panicked) instead of getting its own working binding. One shared, lazily-initialized,
// mutex-guarded instance both avoids the double-bind and serializes access to the underlying
// C library, which isn't safe to call into from multiple threads at once.
static PDFIUM: OnceLock<Mutex<Pdfium>> = OnceLock::new();

fn with_pdfium<T>(f: impl FnOnce(&Pdfium) -> T) -> T {
    let mutex = PDFIUM.get_or_init(|| {
        Mutex::new(Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
                &std::env::var("PDFIUM_DYNAMIC_LIB_PATH").expect("PDFIUM_DYNAMIC_LIB_PATH not set"),
            ))
            .expect("failed to bind to libpdfium — check PDFIUM_DYNAMIC_LIB_PATH"),
        ))
    });
    let pdfium = mutex.lock().unwrap();
    f(&pdfium)
}

fn render_first_page_to_rgba(pdf_path: &std::path::Path) -> image::RgbaImage {
    with_pdfium(|pdfium| {
        let document = pdfium.load_pdf_from_file(pdf_path, None).expect("failed to load rendered PDF");
        let page = document.pages().get(0).expect("PDF has no pages");
        let bitmap = page.render_with_config(&PdfRenderConfig::new().set_target_width(800)).expect("failed to rasterize page");
        bitmap.as_image().expect("failed to convert pdfium bitmap to image").to_rgba8()
    })
}

fn fraction_differing(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "golden and candidate images have different dimensions");
    let total = a.pixels().len() as f64;
    let differing = a.pixels().zip(b.pixels()).filter(|(p1, p2)| p1 != p2).count() as f64;
    differing / total
}

#[test]
fn formatting_fixture_matches_golden_render() {
    let out_path = std::env::temp_dir().join("sardown-visual-formatting.pdf");
    let mut cmd = Command::cargo_bin("sardown").unwrap();
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
#[ignore] // run explicitly: cargo test -p sardown-cli --test visual_regression -- --ignored update_golden
fn update_golden() {
    let out_path = std::env::temp_dir().join("sardown-visual-formatting.pdf");
    let mut cmd = Command::cargo_bin("sardown").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/formatting.md")).arg("-o").arg(&out_path);
    cmd.assert().success();
    let candidate = render_first_page_to_rgba(&out_path);
    candidate.save(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/formatting-page-1.png")).expect("failed to save golden image");
}

#[test]
fn linking_fixture_matches_golden_render() {
    // Regression test: usvg needs a populated, resolvable font database to shape a Mermaid
    // diagram's <text> labels into glyph outlines. With Options::default() (no fonts at all),
    // or a font database whose generic "sans-serif" alias doesn't resolve to any font actually
    // present, every text label is silently dropped -- the diagram's boxes and arrows (pure
    // geometry) still render, but "Parse"/"Layout"/"Emit PDF" do not. This fixture's diagram
    // only requests "sans-serif" as its final fallback family, exactly the failure mode found.
    let out_path = std::env::temp_dir().join("sardown-visual-linking.pdf");
    let mut cmd = Command::cargo_bin("sardown").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linking.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let candidate = render_first_page_to_rgba(&out_path);
    let golden_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/linking-page-1.png");
    let golden =
        image::open(golden_path).unwrap_or_else(|_| panic!("no golden image at {golden_path} — run with --ignored update_linking_golden first")).to_rgba8();

    let diff = fraction_differing(&golden, &candidate);
    assert!(diff < PIXEL_DIFF_THRESHOLD, "rendered output diverged from golden by {:.2}% of pixels", diff * 100.0);
}

/// Fraction of pixels within `(x0, y0, x1, y1)` (in the *same 800px-wide raster* `render_first_page_to_rgba`
/// produces) whose brightest channel is below `180` -- dark enough to be text ink (`#333`) rather
/// than the diagram's light node fill (`#ECECFF`) or purple border (`#9370DB`, whose blue channel
/// alone is 219).
fn dark_pixel_fraction(image: &image::RgbaImage, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> f64 {
    let mut dark = 0u32;
    let mut total = 0u32;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = image.get_pixel(x, y);
            total += 1;
            if p[0].max(p[1]).max(p[2]) < 180 {
                dark += 1;
            }
        }
    }
    dark as f64 / total as f64
}

#[test]
fn mermaid_diagram_node_actually_contains_text_ink() {
    // Regression test for the root cause the golden-image diff above is too coarse to catch:
    // usvg needs a *resolvable* font to shape a diagram's <text> labels into glyph outlines.
    // With no fonts loaded, or a fontdb whose generic "sans-serif" alias doesn't resolve to any
    // font actually present, every text label is silently dropped -- the diagram's boxes/arrows
    // (pure geometry) still render pixel-identical either way, so a whole-page pixel diff barely
    // moves (missing text is a tiny fraction of ~800,000 total page pixels). Checking a crop of
    // just the first node's interior is far more sensitive: empirically, that crop is ~1.7% dark
    // pixels with the "Parse" label rendered, vs ~0.1% with no text at all (residual anti-
    // aliasing noise from the box border/arrow) -- comfortably separated by this threshold.
    let out_path = std::env::temp_dir().join("sardown-visual-linking-textcheck.pdf");
    let mut cmd = Command::cargo_bin("sardown").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linking.md")).arg("-o").arg(&out_path);
    cmd.assert().success();

    let candidate = render_first_page_to_rgba(&out_path);
    // The diagram's first node ("Parse") interior box, in the same 800px-wide raster
    // render_first_page_to_rgba produces -- derived from the diagram's known page placement
    // (x=72pt, y=144pt, unscaled since it's narrower than the content width) plus the node's own
    // rect in the Mermaid-generated SVG (local rect x=-49..49, y=-27..27 around its
    // translate(70.367,35) center), scaled by 800/612 (US Letter page width to raster width).
    let dark_fraction = dark_pixel_fraction(&candidate, (100, 190, 270, 280));
    assert!(dark_fraction > 0.005, "expected the diagram's first node to contain visible text ink, found only {:.3}% dark pixels", dark_fraction * 100.0);
}

#[test]
#[ignore] // run explicitly: cargo test -p sardown-cli --test visual_regression -- --ignored update_linking_golden
fn update_linking_golden() {
    let out_path = std::env::temp_dir().join("sardown-visual-linking.pdf");
    let mut cmd = Command::cargo_bin("sardown").unwrap();
    cmd.arg("render").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linking.md")).arg("-o").arg(&out_path);
    cmd.assert().success();
    let candidate = render_first_page_to_rgba(&out_path);
    candidate.save(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/linking-page-1.png")).expect("failed to save golden image");
}
