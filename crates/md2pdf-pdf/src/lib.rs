mod glyphs;

use anyhow::Context;
use krilla::configure::validate::Archival;
use krilla::configure::{Configuration, ConfigurationBuilder};
use krilla::document::Document;
use krilla::geom::{Point, Size};
use krilla::page::PageSettings;
use krilla::text::Font;
use krilla::Data;
use md2pdf_layout::{PositionedElement, PositionedPage};
use std::collections::HashMap;

const PAGE_WIDTH_PT: f32 = 612.0; // US Letter, matches Phase 1's single fixed page size
const PAGE_HEIGHT_PT: f32 = 792.0;

fn pdf_a2b_configuration() -> anyhow::Result<Configuration> {
    ConfigurationBuilder::new().with_archival_validator(Archival::A2_B).finish().map_err(|e| anyhow::anyhow!("invalid krilla configuration: {e:?}"))
}

pub fn render_pdf(pages: &[PositionedPage], font_data: &fontdb::Database) -> anyhow::Result<Vec<u8>> {
    let mut document = Document::new();
    let _ = pdf_a2b_configuration()?; // wired into Document::new_with in Phase 4; validated here for now

    let page_size = Size::from_wh(PAGE_WIDTH_PT, PAGE_HEIGHT_PT)
        .context("invalid fixed page size constants")?;

    // Cache one krilla::text::Font per fontdb::ID so repeated glyphs on the same page
    // don't reload/re-register the font.
    let mut font_cache: HashMap<fontdb::ID, Font> = HashMap::new();

    for page_data in pages {
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut surface = page.surface();

        for element in &page_data.elements {
            if let PositionedElement::TextRun { x, y, glyphs, font_id, size, .. } = element {
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
                let units_per_em = font.units_per_em();
                let text_for_range = " ".repeat(glyphs.len()); // placeholder text; real text threaded through in Phase 2
                let krilla_glyphs: Vec<_> = glyphs
                    .iter()
                    .enumerate()
                    .map(|(i, g)| glyphs::to_krilla_glyph(g, i..i + 1, units_per_em))
                    .collect();
                surface.draw_glyphs(
                    Point::from_xy(*x, *y),
                    &krilla_glyphs,
                    font,
                    &text_for_range,
                    *size,
                    false, // outlined: false selects the normal (non-Type-3) glyph path
                );
            }
            // Path/VectorGraphic/LinkAnnotation handled starting Phase 2/3/4.
        }
    }

    document.finish().context("krilla failed to serialize the document")
}
