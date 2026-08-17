use crate::{PositionedElement, PositionedGlyph};
use cosmic_text::{Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, Weight};
use sardown_ast::InlineNode;

const PT_TO_PX_SCALE: f32 = 1.0; // 1pt == 1px at our fixed 96/72... kept 1:1 for Phase 1 simplicity

/// Maps a stylesheet's `font_family` string to a cosmic-text `Family`. The five values this
/// project's schema documents as generic keywords map to fontdb's own matching generic-family
/// variants. Anything else is treated as a literal family name -- checked against `db`'s loaded
/// faces first, since an unresolvable name doesn't fail in cosmic-text/fontdb, it silently
/// degrades to whatever font an internal scoring heuristic picks across every loaded face
/// (confirmed against fontdb 0.23's `get_font_matches`/`query` source) -- not a predictable
/// fallback worth depending on. A resolvable check-then-use here makes the fallback explicit and
/// warned instead.
fn resolve_family<'a>(db: &fontdb::Database, requested: &'a str) -> Family<'a> {
    match requested {
        "serif" => Family::Serif,
        "sans-serif" | "sans serif" => Family::SansSerif,
        "monospace" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        name => {
            let known = db.faces().any(|face| face.families.iter().any(|(family_name, _)| family_name.eq_ignore_ascii_case(name)));
            if known {
                Family::Name(name)
            } else {
                eprintln!("warning: unknown font family {name:?}; falling back to a sans-serif font");
                Family::SansSerif
            }
        }
    }
}

pub fn shape_paragraph(font_system: &mut FontSystem, content: &[InlineNode], max_width_pt: f32) -> Vec<PositionedElement> {
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
        .family(resolve_family(font_system.db(), &content[0].style.font_family))
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
            if glyph.glyph_id == 0 {
                // Glyph ID 0 is always ".notdef" (the "tofu box") by OpenType convention --
                // cosmic-text's fallback when NO loaded font covers this character. PDF/A
                // strictly forbids emitting it, and krilla refuses to serialize a document that
                // contains one at all, which would otherwise abort the *entire* render over one
                // unsupported character. Drop it and keep going, matching this project's
                // established convention of skipping the one broken piece of content instead of
                // failing the whole document (dangling links, unsupported diagrams, missing
                // images).
                // cosmic-text's cluster start/end don't always land on a UTF-8 char boundary in
                // `full_text` for characters outside the Basic Multilingual Plane -- when that
                // happens, report the drop without guessing at (and potentially misreporting) an
                // unrelated character.
                match full_text.get(glyph.start..glyph.end) {
                    Some(ch) => eprintln!(
                        "warning: character {ch:?} is not supported by any available font; dropping it \
                         from the output (PDF/A forbids the resulting .notdef glyph)"
                    ),
                    None => eprintln!(
                        "warning: a character is not supported by any available font; dropping it \
                         from the output (PDF/A forbids the resulting .notdef glyph)"
                    ),
                }
                continue;
            }
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
pub fn shape_rich_paragraph(font_system: &mut FontSystem, content: &[InlineNode], max_width_pt: f32, align: Align) -> Vec<ShapedRun> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::with_capacity(content.len());
    let mut rich_text_spans: Vec<(&str, Attrs)> = Vec::with_capacity(content.len());
    let mut offset = 0usize;
    for node in content {
        let attrs = Attrs::new()
            .family(resolve_family(font_system.db(), &node.style.font_family))
            .weight(if node.style.bold { Weight::BOLD } else { Weight::NORMAL })
            .style(if node.style.italic { Style::Italic } else { Style::Normal });
        rich_text_spans.push((node.text.as_str(), attrs));
        spans.push(Span { range: offset..offset + node.text.len(), size: node.style.size, color: node.style.color });
        offset += node.text.len();
    }

    // cosmic-text splits the text passed to `set_rich_text` into separate `BufferLine`s at each
    // '\n' *before* shaping, and `LayoutGlyph::start`/`end` ("index of cluster in original line",
    // per cosmic-text's own doc comment) are relative to that glyph's own BufferLine, resetting
    // to 0 at the start of every line -- not a running offset into the whole rich-text sequence.
    // A code block's tokens (many spans, several embedded newlines from syntect's own per-line
    // tokenization) is exactly the combination that exposes this: `span_index_for` compared a
    // line-relative offset against the globally-accumulated `spans` ranges, resolving to
    // whichever span happened to occupy that same *relative* position on line 0 -- e.g. every
    // BufferLine's first few characters wrongly inherited line 0's own span boundaries, coloring
    // parts of unrelated words. `line_starts[line_i]` recovers each BufferLine's own global
    // starting offset so it can be added back before every span lookup.
    let full_text: String = content.iter().map(|n| n.text.as_str()).collect();
    let mut line_starts = vec![0usize];
    for (i, ch) in full_text.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1);
        }
    }

    let size = content[0].style.size; // buffer-wide metrics still need one size; per-run font
                                      // SIZE variation within one paragraph remains out of
                                      // scope (weight/style/color do not)
    let metrics = Metrics::new(size * PT_TO_PX_SCALE, size * PT_TO_PX_SCALE * 1.4);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_size(Some(max_width_pt * PT_TO_PX_SCALE), None);
    buffer.set_rich_text(rich_text_spans, &Attrs::new(), Shaping::Advanced, None);
    for line in &mut buffer.lines {
        line.set_align(Some(align));
    }
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
        //
        // A line can *also* contain glyphs from more than one font within a single span, when a
        // character isn't covered by the span's primary font and cosmic-text substitutes a
        // fallback font for just that glyph (e.g. an arrow or symbol missing from the main
        // sans-serif face). A `TextRun` carries one `font_id` for all of its glyphs, so without
        // also flushing on a font change, a fallback glyph's ID -- valid only in the fallback
        // font's glyph table -- ends up rendered against whichever font the *last* glyph in the
        // group happened to resolve to, showing an unrelated, effectively random glyph instead.
        let line_text = run.text.to_string();
        let line_offset = line_starts.get(run.line_i).copied().unwrap_or(0);
        let mut current_span: Option<usize> = None;
        let mut current_font_id: Option<fontdb::ID> = None;
        let mut current_group_start_x: f32 = 0.0;
        let mut current_glyphs: Vec<PositionedGlyph> = Vec::new();

        for glyph in run.glyphs {
            // Only for span lookup: `glyph.start`/`end` themselves stay line-relative below,
            // matching `line_text` (== this TextRun's own `text` field), since krilla correlates
            // `PositionedGlyph::cluster` against that same string for ToUnicode text extraction --
            // not against this function's own internal global-offset bookkeeping.
            let global_start = line_offset + glyph.start;
            let span_index = span_index_for(global_start);
            if glyph.glyph_id == 0 {
                // See shape_paragraph's identical check for why .notdef glyphs are dropped
                // rather than rendered: PDF/A forbids them, and krilla refuses to serialize a
                // document containing one at all.
                let span = &spans[span_index];
                let local_start = global_start.saturating_sub(span.range.start);
                let local_end = (line_offset + glyph.end).saturating_sub(span.range.start);
                // See shape_paragraph's identical fallback for why a missing char boundary isn't
                // guessed at here.
                match content[span_index].text.get(local_start..local_end) {
                    Some(ch) => eprintln!(
                        "warning: character {ch:?} is not supported by any available font; dropping it \
                         from the output (PDF/A forbids the resulting .notdef glyph)"
                    ),
                    None => eprintln!(
                        "warning: a character is not supported by any available font; dropping it \
                         from the output (PDF/A forbids the resulting .notdef glyph)"
                    ),
                }
                continue;
            }
            let span_changed = current_span.is_some() && current_span != Some(span_index);
            let font_changed = current_font_id.is_some() && current_font_id != Some(glyph.font_id);
            if span_changed || font_changed {
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
