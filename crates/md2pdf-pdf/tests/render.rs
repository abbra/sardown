use md2pdf_layout::{PositionedElement, PositionedGlyph, PositionedPage};
use md2pdf_pdf::render_pdf;

fn test_font_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db
}

#[test]
fn renders_a_single_page_with_one_text_run_to_valid_pdf_bytes() {
    let db = test_font_db();
    let font_id = db.faces().next().expect("no faces in test font").id;

    // A single glyph run is enough to prove the krilla bridge works end-to-end;
    // exact glyph IDs come from Task 7's shape_paragraph in real use.
    let page = PositionedPage {
        page_number: 0,
        elements: vec![PositionedElement::TextRun {
            x: 72.0,
            y: 72.0,
            glyphs: vec![PositionedGlyph { glyph_id: 3, x: 0.0, y: 0.0, x_advance: 10.0, cluster: 0..1 }],
            text: "x".to_string(),
            font_id,
            size: 12.0,
            color: [0, 0, 0],
        }],
    };

    let pdf_bytes = render_pdf(&[page], &db, &ImageTable::new(), &DiagramTable::new(), &AnchorTable::new())
        .expect("render_pdf failed");

    assert!(pdf_bytes.starts_with(b"%PDF-"), "output does not start with a PDF header");
    let doc = lopdf::Document::load_mem(&pdf_bytes).expect("krilla output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 1, "expected exactly one page");
}

use md2pdf_layout::{DecodedImage, ImageTable, PathCommand, StrokeStyle};

#[test]
fn renders_a_page_with_a_stroked_path_and_a_raster_image() {
    let db = test_font_db();
    let mut images = ImageTable::new();
    images.insert("dot.png".to_string(), DecodedImage { rgba8: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255], width: 2, height: 2 });

    let page = PositionedPage {
        page_number: 0,
        elements: vec![
            PositionedElement::Path {
                points: vec![PathCommand::MoveTo(10.0, 10.0), PathCommand::LineTo(100.0, 10.0)],
                fill: None,
                stroke: Some(StrokeStyle { color: [0, 0, 0], width: 1.0 }),
            },
            PositionedElement::RasterImage { x: 10.0, y: 20.0, width: 50.0, height: 50.0, image_id: "dot.png".to_string() },
        ],
    };

    let pdf_bytes = render_pdf(&[page], &db, &images, &DiagramTable::new(), &AnchorTable::new())
        .expect("render_pdf failed");
    let doc = lopdf::Document::load_mem(&pdf_bytes).expect("output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 1);
}

#[test]
fn text_after_a_stroked_path_is_not_drawn_in_fill_and_stroke_mode() {
    // Regression test: krilla's `Surface` keeps `set_stroke`'s value active across draw calls,
    // the same way it keeps `set_fill`'s. A stroked Path (thematic breaks, blockquote borders,
    // table grid lines) left its stroke active on the surface, so any TextRun drawn afterward
    // came out in PDF text-rendering mode 2 (fill *and* stroke) instead of mode 0 (fill only) --
    // every glyph traced in the leftover stroke color, visible as faint, washed-out text instead
    // of solid black.
    let db = test_font_db();
    let font_id = db.faces().next().unwrap().id;
    let page = PositionedPage {
        page_number: 0,
        elements: vec![
            PositionedElement::Path {
                points: vec![PathCommand::MoveTo(10.0, 10.0), PathCommand::LineTo(100.0, 10.0)],
                fill: None,
                stroke: Some(StrokeStyle { color: [200, 200, 200], width: 1.0 }),
            },
            PositionedElement::TextRun {
                x: 72.0,
                y: 72.0,
                glyphs: vec![PositionedGlyph { glyph_id: 3, x: 0.0, y: 0.0, x_advance: 10.0, cluster: 0..1 }],
                text: "x".to_string(),
                font_id,
                size: 12.0,
                color: [0, 0, 0],
            },
        ],
    };

    let pdf_bytes = render_pdf(&[page], &db, &ImageTable::new(), &DiagramTable::new(), &AnchorTable::new())
        .expect("render_pdf failed");
    let doc = lopdf::Document::load_mem(&pdf_bytes).expect("output is not a valid PDF");
    let page_id = *doc.get_pages().values().next().expect("expected one page");
    let content = doc.get_page_content(page_id);
    let content = String::from_utf8_lossy(&content);
    assert!(!content.contains("2 Tr"), "text should not be drawn in fill+stroke mode after a stroked Path; content stream:\n{content}");
}

use md2pdf_ast::LinkTarget;
use md2pdf_enrich::{CompiledDiagram, DiagramTable};
use md2pdf_layout::{AnchorPosition, AnchorTable, Rect};

fn valid_test_svg() -> String {
    // Minimal but well-formed SVG usvg can parse — a single rectangle.
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
        <rect x="0" y="0" width="100" height="50" fill="#ff0000"/>
    </svg>"##
        .to_string()
}

#[test]
fn renders_a_page_with_a_diagram_and_both_link_kinds() {
    let db = test_font_db();
    let mut diagrams = DiagramTable::new();
    diagrams.insert("d1".to_string(), CompiledDiagram { svg: valid_test_svg(), width: 100.0, height: 50.0 });

    let mut anchors = AnchorTable::new();
    anchors.insert("target".to_string(), AnchorPosition { page: 0, x: 72.0, y: 100.0 });

    let page = PositionedPage {
        page_number: 0,
        elements: vec![
            PositionedElement::VectorGraphic { x: 10.0, y: 10.0, width: 100.0, height: 50.0, diagram_id: "d1".to_string() },
            PositionedElement::LinkAnnotation {
                rect: Rect { x: 10.0, y: 70.0, width: 80.0, height: 12.0 },
                destination: LinkTarget::ExternalUrl("https://example.com".to_string()),
            },
            PositionedElement::LinkAnnotation {
                rect: Rect { x: 10.0, y: 90.0, width: 80.0, height: 12.0 },
                destination: LinkTarget::InternalAnchor("target".to_string()),
            },
        ],
    };

    let pdf_bytes = render_pdf(&[page], &db, &ImageTable::new(), &diagrams, &anchors)
        .expect("render_pdf failed");
    let doc = lopdf::Document::load_mem(&pdf_bytes).expect("output is not a valid PDF");
    assert_eq!(doc.get_pages().len(), 1);
}

#[test]
fn dangling_internal_anchor_is_skipped_not_errored() {
    let db = test_font_db();
    let page = PositionedPage {
        page_number: 0,
        elements: vec![PositionedElement::LinkAnnotation {
            rect: Rect { x: 10.0, y: 10.0, width: 80.0, height: 12.0 },
            destination: LinkTarget::InternalAnchor("does-not-exist".to_string()),
        }],
    };
    let result = render_pdf(&[page], &db, &ImageTable::new(), &DiagramTable::new(), &AnchorTable::new());
    assert!(result.is_ok(), "a dangling internal link should be silently skipped, not fail the whole render");
}

#[test]
fn embedded_font_is_subsetted_not_fully_embedded() {
    let db = test_font_db();
    let font_id = db.faces().next().unwrap().id;
    let source_font_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../md2pdf-layout/tests/fixtures/DroidSans.ttf")).unwrap();

    let page = PositionedPage {
        page_number: 0,
        elements: vec![PositionedElement::TextRun {
            x: 72.0,
            y: 72.0,
            glyphs: vec![PositionedGlyph { glyph_id: 3, x: 0.0, y: 0.0, x_advance: 10.0, cluster: 0..1 }],
            text: "x".to_string(),
            font_id,
            size: 12.0,
            color: [0, 0, 0],
        }],
    };
    let pdf_bytes = render_pdf(
        &[page],
        &db,
        &ImageTable::new(),
        &DiagramTable::new(),
        &AnchorTable::new(),
    )
    .unwrap();

    // krilla doesn't set the (spec-optional) `Length1` key on FontFile2 streams, so the only
    // reliable way to find the embedded font program is to follow FontDescriptor -> FontFile2
    // directly, confirmed by inspecting the actual object graph krilla produces.
    let doc = lopdf::Document::load_mem(&pdf_bytes).unwrap();
    let embedded_font_bytes: usize = doc
        .objects
        .values()
        .filter_map(|obj| obj.as_dict().ok())
        .filter(|dict| dict.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"FontDescriptor"))
        .filter_map(|dict| dict.get(b"FontFile2").ok())
        .filter_map(|font_file| font_file.as_reference().ok())
        .filter_map(|reference| doc.get_object(reference).ok())
        .filter_map(|obj| obj.as_stream().ok())
        .map(|stream| stream.content.len())
        .sum();

    assert!(embedded_font_bytes > 0, "no embedded font program (FontDescriptor -> FontFile2) found in output PDF");
    assert!(
        embedded_font_bytes < source_font_bytes.len() / 2,
        "embedded font ({embedded_font_bytes} bytes) is not meaningfully smaller than the full source font \
         ({} bytes) — subsetting does not appear to be occurring",
        source_font_bytes.len()
    );
}
