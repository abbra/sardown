mod glyphs;
mod links;
mod paths;

use anyhow::Context;
use krilla::configure::validate::Archival;
use krilla::configure::{Configuration, ConfigurationBuilder};
use krilla::document::Document;
use krilla::geom::{Point, Size, Transform};
use krilla::image::Image;
use krilla::page::PageSettings;
use krilla::text::Font;
use krilla::{Data, SerializeSettings};
use krilla_svg::SurfaceExt;
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{AnchorTable, ImageTable, PositionedElement, PositionedPage};
use std::collections::HashMap;

const PAGE_WIDTH_PT: f32 = 612.0; // US Letter, matches Phase 1's single fixed page size
const PAGE_HEIGHT_PT: f32 = 792.0;

fn pdf_a2b_configuration() -> anyhow::Result<Configuration> {
    ConfigurationBuilder::new().with_archival_validator(Archival::A2_B).finish().map_err(|e| anyhow::anyhow!("invalid krilla configuration: {e:?}"))
}

pub fn render_pdf(
    pages: &[PositionedPage],
    font_data: &fontdb::Database,
    images: &ImageTable,
    diagrams: &DiagramTable,
    anchors: &AnchorTable,
) -> anyhow::Result<Vec<u8>> {
    let configuration = pdf_a2b_configuration()?;
    let settings = SerializeSettings { configuration, ..Default::default() };
    let mut document = Document::new_with(settings);

    let page_size = Size::from_wh(PAGE_WIDTH_PT, PAGE_HEIGHT_PT)
        .context("invalid fixed page size constants")?;

    // Cache one krilla::text::Font per fontdb::ID so repeated glyphs on the same page
    // don't reload/re-register the font.
    let mut font_cache: HashMap<fontdb::ID, Font> = HashMap::new();

    for page_data in pages {
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut pending_annotations = Vec::new();

        {
            let mut surface = page.surface();
            for element in &page_data.elements {
                match element {
                    PositionedElement::TextRun { x, y, glyphs, text, font_id, size, color } => {
                        let font = match font_cache.get(font_id) {
                            Some(f) => f.clone(),
                            None => {
                                let font = font_data
                                    .with_face_data(*font_id, |data, index| Font::new(Data::from(data.to_vec()), index))
                                    .context("font id not found in fontdb")?
                                    .context("krilla::text::Font::new rejected the font data")?;
                                font_cache.insert(*font_id, font.clone());
                                font
                            }
                        };
                        let krilla_glyphs: Vec<_> = glyphs.iter().map(|g| glyphs::to_krilla_glyph(g, *size)).collect();
                        // `draw_glyphs` has no color parameter of its own — it fills with
                        // whatever `set_fill` last set on the surface, so every text run must set
                        // its own color explicitly or it silently inherits the last shape's fill
                        // (e.g. a code block's light-gray background box drawn earlier on the page).
                        surface.set_fill(Some(paths::krilla_fill(*color)));
                        // Same issue for stroke: any earlier stroked Path (a thematic break, a
                        // blockquote border, a table grid line) leaves its stroke active on the
                        // surface. Left set, text drawn afterward is emitted in PDF text-rendering
                        // mode 2 (fill *and* stroke) instead of mode 0 (fill only), tracing every
                        // glyph in that leftover stroke color/width -- visible as faint, "hollow"-
                        // looking text instead of solid black. Text never wants a stroke of its
                        // own, so always clear it before drawing.
                        surface.set_stroke(None);
                        surface.draw_glyphs(
                            Point::from_xy(*x, *y),
                            &krilla_glyphs,
                            font,
                            text,
                            *size,
                            false, // outlined: false selects the normal (non-Type-3) glyph path
                        );
                    }
                    PositionedElement::Path { points, fill, stroke } => {
                        let path = paths::build_path(points);
                        surface.set_fill(fill.map(paths::krilla_fill));
                        surface.set_stroke(stroke.as_ref().map(paths::krilla_stroke));
                        surface.draw_path(&path);
                    }
                    PositionedElement::RasterImage { x, y, width, height, image_id } => {
                        if let Some(decoded) = images.get(image_id) {
                            let image = Image::from_rgba8(decoded.rgba8.clone(), decoded.width, decoded.height);
                            let size = Size::from_wh(*width, *height).context("invalid image size")?;
                            surface.push_transform(&Transform::from_translate(*x, *y));
                            surface.draw_image(image, size);
                            surface.pop();
                        }
                    }
                    PositionedElement::VectorGraphic { x, y, width, height, diagram_id } => {
                        if let Some(diagram) = diagrams.get(diagram_id) {
                            match usvg::Tree::from_str(&diagram.svg, &usvg::Options::default()) {
                                Ok(tree) => {
                                    let size = Size::from_wh(*width, *height).context("invalid diagram size")?;
                                    surface.push_transform(&Transform::from_translate(*x, *y));
                                    surface.draw_svg(&tree, size, krilla_svg::SvgSettings::default());
                                    surface.pop();
                                }
                                Err(e) => {
                                    eprintln!("warning: failed to re-parse diagram '{diagram_id}' SVG at render time: {e}")
                                }
                            }
                        }
                    }
                    PositionedElement::LinkAnnotation { rect, destination } => {
                        if let Some(annotation) = links::build_annotation(rect, destination, anchors) {
                            pending_annotations.push(annotation);
                        }
                    }
                }
            }
        } // `surface` dropped here, releasing its borrow of `page`

        for annotation in pending_annotations {
            page.add_annotation(annotation);
        }
    }

    document.finish().context("krilla failed to serialize the document")
}
