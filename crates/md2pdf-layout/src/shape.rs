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
            glyphs.push(PositionedGlyph { glyph_id: glyph.glyph_id, x: glyph.x, y: glyph.y, x_advance: glyph.w, cluster: glyph.start..glyph.end });
        }
        let Some(font_id) = font_id else { continue };
        elements.push(PositionedElement::TextRun { x: 0.0, y: run.line_y, glyphs, text: run.text.to_string(), font_id, size, color });
    }
    elements
}

/// One shaped glyph run, tagged with the index into the original `content` slice it came from.
pub struct ShapedRun {
    pub element: PositionedElement, // always PositionedElement::TextRun
    pub source_index: usize,
}

struct Span {
    range: std::ops::Range<usize>,
    size: f32,
    color: [u8; 3],
}

/// Like `shape_paragraph`, but preserves per-`InlineNode` style/color boundaries even when
/// multiple runs share a visual line — needed wherever bold/italic/color/links can appear mixed
/// within one paragraph or code block line. Uses cosmic-text's `set_rich_text` (one span per
/// `InlineNode`) and recovers which span each glyph came from via `LayoutGlyph`'s `start`/`end`
/// cluster fields against precomputed per-span byte ranges — no dependency on any less-certain
/// "glyph metadata echo" API.
pub fn shape_rich_paragraph(font_system: &mut FontSystem, content: &[InlineNode], max_width_pt: f32) -> Vec<ShapedRun> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::with_capacity(content.len());
    let mut rich_text_spans: Vec<(&str, Attrs)> = Vec::with_capacity(content.len());
    let mut offset = 0usize;
    for node in content {
        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(if node.style.bold { Weight::BOLD } else { Weight::NORMAL })
            .style(if node.style.italic { Style::Italic } else { Style::Normal });
        rich_text_spans.push((node.text.as_str(), attrs));
        spans.push(Span { range: offset..offset + node.text.len(), size: node.style.size, color: node.style.color });
        offset += node.text.len();
    }

    let size = content[0].style.size; // buffer-wide metrics still need one size; per-run font
                                      // SIZE variation within one paragraph remains out of
                                      // scope (weight/style/color do not)
    let metrics = Metrics::new(size * PT_TO_PX_SCALE, size * PT_TO_PX_SCALE * 1.4);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_size(Some(max_width_pt * PT_TO_PX_SCALE), None);
    buffer.set_rich_text(rich_text_spans, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let span_index_for = |cluster_start: usize| spans.iter().position(|s| s.range.contains(&cluster_start)).unwrap_or(spans.len().saturating_sub(1));

    let mut runs = Vec::new();
    for run in buffer.layout_runs() {
        // A single visual line can contain glyphs from more than one span (e.g. "plain **bold**
        // plain" on one line) — flush a ShapedRun each time the source span changes. Each flushed
        // group's `x` must be the *first* glyph's absolute along-the-line position, not 0.0:
        // krilla's draw_glyphs positions glyphs by cumulative advance starting from the run's
        // `start` point, so every group needs its own correct starting offset or later groups on
        // the same line would all draw starting from the same x and overlap.
        let line_text = run.text.to_string();
        let mut current_span: Option<usize> = None;
        let mut current_font_id: Option<fontdb::ID> = None;
        let mut current_group_start_x: f32 = 0.0;
        let mut current_glyphs: Vec<PositionedGlyph> = Vec::new();

        for glyph in run.glyphs {
            let span_index = span_index_for(glyph.start);
            if current_span.is_some() && current_span != Some(span_index) {
                let span = &spans[current_span.unwrap()];
                runs.push(ShapedRun {
                    source_index: current_span.unwrap(),
                    element: PositionedElement::TextRun {
                        x: current_group_start_x,
                        y: run.line_y,
                        glyphs: std::mem::take(&mut current_glyphs),
                        text: line_text.clone(),
                        font_id: current_font_id.unwrap(),
                        size: span.size,
                        color: span.color,
                    },
                });
            }
            if current_glyphs.is_empty() {
                current_group_start_x = glyph.x;
            }
            current_span = Some(span_index);
            current_font_id = Some(glyph.font_id);
            current_glyphs.push(PositionedGlyph { glyph_id: glyph.glyph_id, x: glyph.x, y: glyph.y, x_advance: glyph.w, cluster: glyph.start..glyph.end });
        }
        if let (Some(span_index), Some(font_id)) = (current_span, current_font_id) {
            let span = &spans[span_index];
            runs.push(ShapedRun {
                source_index: span_index,
                element: PositionedElement::TextRun {
                    x: current_group_start_x,
                    y: run.line_y,
                    glyphs: current_glyphs,
                    text: line_text,
                    font_id,
                    size: span.size,
                    color: span.color,
                },
            });
        }
    }
    runs
}
