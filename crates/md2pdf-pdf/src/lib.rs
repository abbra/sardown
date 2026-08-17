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
use krilla::text::Font;
use krilla::{Data, SerializeSettings};
use krilla_svg::SurfaceExt;
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{AnchorTable, ImageTable, PositionedElement, PositionedPage};
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
    toc_entries: &[md2pdf_layout::TocEntry],
) -> anyhow::Result<Vec<u8>> {
    let configuration = pdf_a2b_configuration()?;
    let settings = SerializeSettings { configuration, ..Default::default() };
    let mut document = Document::new_with(settings);

    let page_size = Size::from_wh(page_width_pt, page_height_pt).context("invalid page size")?;

    // Cache one krilla::text::Font per fontdb::ID so repeated glyphs on the same page
    // don't reload/re-register the font.
    let mut font_cache: HashMap<fontdb::ID, Font> = HashMap::new();
    // Built once (not per-diagram): cloning the database is far cheaper than re-scanning fonts
    // from disk, and a document can contain many diagrams.
    let svg_options = svg_render_options(font_data);

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
                            match usvg::Tree::from_str(&diagram.svg, &svg_options) {
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

/// usvg needs a font database to shape `<text>` elements into glyph outlines. Reuses the
/// document's own font database (already respects `typography.font_dirs`/`use_system_fonts`, and
/// was already loaded once for the rest of the document's text) instead of building a fresh,
/// system-fonts-only one from scratch -- cheaper (cloning metadata beats re-scanning disk), and
/// gives diagram text access to the same custom fonts the rest of the document uses instead of
/// silently ignoring `font_dirs` for diagrams specifically.
///
/// Real-world SVGs (e.g. Graphviz output, which commonly emits
/// `font-family="Helvetica,sans-Serif"` -- note the capital S) often name literal fonts that
/// aren't installed, and usvg's own font-family parser only recognizes the lowercase CSS generic
/// keywords, so "sans-Serif" parses as a literal name too and also fails to match. usvg's default
/// font selector unconditionally appends `fontdb::Family::Serif` as its last-resort fallback once
/// every requested family fails -- but a fontconfig-advertised generic alias (e.g. serif ->
/// "FreeSerif") can point at a font that isn't actually installed, and `fontdb::Database::query`
/// has no further fallback of its own once every requested family fails to match a loaded face.
/// So both the serif and sans-serif generic aliases are repointed at whatever's actually loaded,
/// guaranteeing usvg's last-resort fallback always resolves to a real, usable font instead of
/// silently dropping the text -- this was a real, reported bug: diagram shapes rendered fine
/// (pure geometry, no font dependency) while every text label silently vanished.
fn svg_render_options(font_data: &fontdb::Database) -> usvg::Options<'static> {
    let mut fontdb = font_data.clone();
    ensure_resolvable_generic_families(&mut fontdb);
    usvg::Options { fontdb: std::sync::Arc::new(fontdb), ..Default::default() }
}

fn ensure_resolvable_generic_families(fontdb: &mut fontdb::Database) {
    let Some(fallback) = fontdb.faces().next().and_then(|face| face.families.first().map(|(name, _)| name.clone())) else {
        return;
    };
    for family in [fontdb::Family::Serif, fontdb::Family::SansSerif] {
        let alias = fontdb.family_name(&family).to_string();
        let alias_resolves = fontdb.faces().any(|face| face.families.iter().any(|(name, _)| *name == alias));
        if alias_resolves {
            continue;
        }
        match family {
            fontdb::Family::Serif => fontdb.set_serif_family(fallback.clone()),
            fontdb::Family::SansSerif => fontdb.set_sans_serif_family(fallback.clone()),
            _ => unreachable!("only Serif and SansSerif are iterated above"),
        }
    }
}
