use crate::{PositionedElement, PositionedGlyph};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, Weight};
use md2pdf_ast::InlineNode;

const PT_TO_PX_SCALE: f32 = 1.0; // 1pt == 1px at our fixed 96/72... kept 1:1 for Phase 1 simplicity

pub fn shape_paragraph(
    font_system: &mut FontSystem,
    content: &[InlineNode],
    max_width_pt: f32,
) -> Vec<PositionedElement> {
    if content.is_empty() {
        return Vec::new();
    }

    // Phase 1 scope: single style per paragraph (first run's size), single font family.
    // Per-run bold/italic and multi-color runs are Phase 2 work (requires splitting each
    // cosmic-text line into per-attrs-span TextRuns instead of one TextRun per line).
    let size = content[0].style.size;
    let metrics = Metrics::new(size * PT_TO_PX_SCALE, size * PT_TO_PX_SCALE * 1.4);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_size(Some(max_width_pt * PT_TO_PX_SCALE), None);

    let full_text: String = content.iter().map(|n| n.text.as_str()).collect::<Vec<_>>().join("");
    let attrs = Attrs::new()
        .family(Family::SansSerif)
        .weight(if content[0].style.bold { Weight::BOLD } else { Weight::NORMAL })
        .style(if content[0].style.italic { Style::Italic } else { Style::Normal });
    buffer.set_text(&full_text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let color = content[0].style.color;
    let mut elements = Vec::new();
    for run in buffer.layout_runs() {
        let mut glyphs = Vec::with_capacity(run.glyphs.len());
        let mut font_id = None;
        for glyph in run.glyphs {
            font_id.get_or_insert(glyph.font_id);
            glyphs.push(PositionedGlyph {
                glyph_id: glyph.glyph_id,
                x: glyph.x,
                y: glyph.y,
                x_advance: glyph.w,
            });
        }
        let Some(font_id) = font_id else { continue };
        elements.push(PositionedElement::TextRun {
            x: 0.0,
            y: run.line_y,
            glyphs,
            font_id,
            size,
            color,
        });
    }
    elements
}
