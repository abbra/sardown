mod glyphs;
mod links;
mod paths;

use anyhow::Context;
use krilla::configure::validate::Archival;
use krilla::configure::{Configuration, ConfigurationBuilder};
use krilla::destination::XyzDestination;
use krilla::document::Document;
use krilla::geom::{Point, Size, Transform};
use krilla::image::Image;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::text::{Font, KrillaGlyph};
use krilla::{Data, SerializeSettings};
use krilla_svg::SurfaceExt;
use sardown_enrich::DiagramTable;
use sardown_layout::{AnchorTable, ImageTable, PositionedElement, PositionedPage};
use std::collections::HashMap;

fn pdf_a2b_configuration() -> anyhow::Result<Configuration> {
    ConfigurationBuilder::new().with_archival_validator(Archival::A2_B).finish().map_err(|e| anyhow::anyhow!("invalid krilla configuration: {e:?}"))
}

#[allow(clippy::too_many_arguments)]
pub fn render_pdf(
    pages: &[PositionedPage],
    font_data: &fontdb::Database,
    images: &ImageTable,
    diagrams: &DiagramTable,
    anchors: &AnchorTable,
    page_width_pt: f32,
    page_height_pt: f32,
    toc_entries: &[sardown_layout::TocEntry],
) -> anyhow::Result<Vec<u8>> {
    let configuration = pdf_a2b_configuration()?;
    let settings = SerializeSettings { configuration, ..Default::default() };
    let mut document = Document::new_with(settings);

    let page_size = Size::from_wh(page_width_pt, page_height_pt).context("invalid page size")?;

    // Cache one krilla::text::Font per fontdb::ID so repeated glyphs on the same page
    // don't reload/re-register the font.
    let mut font_cache: HashMap<fontdb::ID, Font> = HashMap::new();

    // Vector diagrams arrive already parsed into render-ready `usvg::Tree`s -- built once per
    // document against the document's own fontdb (see `sardown_enrich::svg_tree_options`) -- so
    // this is just a keyed clone and no SVG markup is ever parsed inside emission. Parse
    // failures are reported once at compile/collection time instead of here.
    let mut svg_cache: HashMap<&str, usvg::Tree> = HashMap::new();
    for (diagram_id, diagram) in diagrams {
        svg_cache.insert(diagram_id, diagram.tree.clone());
    }

    // Raster images are converted into a `krilla::Image` lazily, on their first placement:
    // `Image::from_rgba8` copies the whole pixel buffer, and images whose every placement was
    // dropped before emission ever saw them (pagination page-breaks, slides auto-shrink
    // retries) would otherwise pay that copy for nothing. A referenced image is still converted
    // exactly once per document -- not once per placement -- and `krilla::Image` is
    // `Arc`-backed, so cloning the entry per placement stays cheap.
    let mut raster_cache: HashMap<&str, Image> = HashMap::new();
    // Reused across every TextRun in the document: one scratch buffer instead of one fresh
    // Vec allocation per emitted text run.
    let mut krilla_glyphs: Vec<KrillaGlyph> = Vec::new();
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
                        krilla_glyphs.clear();
                        krilla_glyphs.extend(glyphs.iter().map(|g| glyphs::to_krilla_glyph(g, *size)));
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
                        // Convert on first placement (see the raster_cache comment above); an id
                        // with no decoded entry draws nothing, exactly as before.
                        if !raster_cache.contains_key(image_id.as_str()) {
                            if let Some(decoded) = images.get(image_id.as_str()) {
                                let image = Image::from_rgba8((*decoded.rgba8).clone(), decoded.width, decoded.height);
                                raster_cache.insert(image_id.as_str(), image);
                            }
                        }
                        if let Some(image) = raster_cache.get(image_id.as_str()) {
                            let size = Size::from_wh(*width, *height).context("invalid image size")?;
                            surface.push_transform(&Transform::from_translate(*x, *y));
                            surface.draw_image(image.clone(), size);
                            surface.pop();
                        }
                    }
                    PositionedElement::VectorGraphic { x, y, width, height, diagram_id } => {
                        if let Some(tree) = svg_cache.get(diagram_id.as_str()) {
                            let size = Size::from_wh(*width, *height).context("invalid diagram size")?;
                            surface.push_transform(&Transform::from_translate(*x, *y));
                            surface.draw_svg(tree, size, krilla_svg::SvgSettings::default());
                            surface.pop();
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

    if !toc_entries.is_empty() {
        let mut outline = Outline::new();
        let mut current_top_level: Option<OutlineNode> = None;
        for entry in toc_entries {
            let Some(anchor) = anchors.get(&entry.id) else { continue };
            let destination = XyzDestination::new(anchor.page, Point::from_xy(anchor.x, anchor.y));
            let node = OutlineNode::new(entry.text.clone(), destination);
            if entry.level <= 1 {
                if let Some(finished) = current_top_level.take() {
                    outline.push_child(finished);
                }
                current_top_level = Some(node);
            } else if let Some(parent) = current_top_level.as_mut() {
                parent.push_child(node);
            } else {
                // A deeper-level entry with no preceding top-level entry (shouldn't happen given
                // TOC generation always starts from level 1, but degrade to a flat top-level
                // entry rather than dropping it).
                outline.push_child(node);
            }
        }
        if let Some(finished) = current_top_level.take() {
            outline.push_child(finished);
        }
        document.set_outline(outline);
    }

    document.finish().context("krilla failed to serialize the document")
}
