use crate::{PositionedElement, PositionedGlyph};
use cosmic_text::{Align, Attrs, Buffer, Family, Feature, FeatureTag, FontFeatures, FontSystem, Metrics, Shaping, Style, Weight};
use sardown_ast::{HighlightedToken, InlineNode, TextStyle};
use std::collections::HashMap;

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
    // A literal family name (the `name =>` arm below) is resolved by scanning EVERY loaded face's
    // alias list -- a full `db.faces()` pass with case-insensitive comparison. That depends only on
    // the font set, which isn't mutated once layout starts, so cache it per (font database pointer,
    // name) in a thread-local: each distinct name scans all faces at most ONCE instead of every call
    // (`shape_rich_paragraph` calls this once per inline node, so one code block's many tokens would
    // otherwise re-scan the whole DB for the same few names). Keyed by the database instance pointer
    // so separate font systems never share entries. Mirrors `MONOSPACE_ADVANCE_CACHE`. (The generic-
    // keyword arms above short-circuit before any scan, exactly as before.)
    thread_local! {
        static FAMILY_KNOWN_CACHE: std::cell::RefCell<HashMap<usize, HashMap<String, bool>>> = Default::default();
    }
    match requested {
        "serif" => Family::Serif,
        "sans-serif" | "sans serif" => Family::SansSerif,
        "monospace" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        name => {
            let db_key = db as *const fontdb::Database as usize;
            let known = FAMILY_KNOWN_CACHE.with(|outer| {
                let mut entries = outer.borrow_mut();
                match entries.entry(db_key).or_default().get(name) {
                    Some(&known) => known,
                    None => {
                        let k = db.faces().any(|face| face.families.iter().any(|(family_name, _)| family_name.eq_ignore_ascii_case(name)));
                        if !k {
                            eprintln!("warning: unknown font family {name:?}; falling back to a sans-serif font");
                        }
                        entries.entry(db_key).or_default().insert(name.to_owned(), k);
                        k
                    }
                }
            });
            if known {
                Family::Name(name)
            } else {
                // Unknown family: warned on the first (miss) lookup above; cache-hit reuses skip it.
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

/// A generous, finite width (rather than `f32::MAX`) for `measure_widest_line_pt`'s unconstrained
/// shaping pass -- keeps `Buffer`'s internal wrap-width arithmetic away from any overflow edge
/// case, while still being far wider than any real code line needs.
const UNCONSTRAINED_WIDTH_PT: f32 = 1_000_000.0;

/// The natural (unwrapped) width, in points, of the widest `\n`-delimited line in `text` when
/// shaped at `size`pt in `font_family` -- used by code blocks' `shrink_to_fit` to decide whether a
/// block's font needs to shrink to keep its longest line from wrapping. Glyph advance widths scale
/// linearly with font size for a fixed font and text, so callers can measure once at the
/// configured size and derive the scale factor needed for any other size by division, rather than
/// re-shaping per candidate size the way `sardown-slides`' iterative whole-page shrink search does
/// -- that search exists because a slide's wrapped line *count* changes non-linearly with scale,
/// which doesn't apply here: only the widest single line matters, and its width is exact.
pub fn measure_widest_line_pt(font_system: &mut FontSystem, text: &str, size: f32, font_family: &str) -> f32 {
    let node = InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size, color: [0, 0, 0], font_family: font_family.into() },
        link_target: None,
    };
    shape_paragraph(font_system, std::slice::from_ref(&node), UNCONSTRAINED_WIDTH_PT)
        .iter()
        .map(|e| match e {
            PositionedElement::TextRun { glyphs, .. } => glyphs.iter().map(|g| g.x_advance).sum(),
            _ => 0.0,
        })
        .fold(0.0_f32, f32::max)
}

/// The advance width, in points, of one character in `font_family` at `size`pt -- `Some` only
/// when the resolved face is genuinely monospaced, in which case a line's natural (unwrapped)
/// width is exactly `char_count * advance` and measuring it needs no shaping pass at all. Probes
/// two characters that are maximally different in width in any proportional face ("m" and "W")
/// and compares their shaped advances: a difference of more than a hair means the face is not
/// monospaced and the estimate is unusable (callers fall back to `measure_widest_line_pt`'s full
/// shaping pass). Cached per (family, size) in a thread-local -- a book's code blocks all share
/// one or a few (family, size) pairs, so the probe shapes once per pair, not once per block.
pub fn monospace_advance_pt(font_system: &mut FontSystem, font_family: &str, size: f32) -> Option<f32> {
    thread_local! {
        static MONOSPACE_ADVANCE_CACHE: std::cell::RefCell<HashMap<(String, u32), Option<f32>>> = Default::default();
    }
    let key = (font_family.to_string(), size.to_bits());
    if let Some(hit) = MONOSPACE_ADVANCE_CACHE.with(|c| c.borrow().get(&key).copied()) {
        return hit;
    }
    let probe = InlineNode {
        text: "mW".to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size, color: [0, 0, 0], font_family: font_family.into() },
        link_target: None,
    };
    let mut advances = Vec::new();
    for element in shape_paragraph(font_system, std::slice::from_ref(&probe), UNCONSTRAINED_WIDTH_PT) {
        if let PositionedElement::TextRun { glyphs, .. } = element {
            for g in glyphs {
                advances.push(g.x_advance);
            }
        }
    }
    let result = match (advances.first().copied(), advances.get(1).copied()) {
        (Some(a), Some(b)) if (a - b).abs() <= 0.01 => Some(a),
        _ => None,
    };
    MONOSPACE_ADVANCE_CACHE.with(|c| {
        c.borrow_mut().insert(key, result);
    });
    result
}

/// The natural (unwrapped) width, in points, of the widest `\n`-delimited line among a code
/// block's `tokens` -- without any shaping pass. A monospaced face advances every character by
/// the same amount, so a line's width is exactly its character count times that advance; the
/// count is a plain string scan over the tokens' text. Returns `None` when the face is not
/// monospaced (or the probe was inconclusive) -- callers then fall back to
/// `measure_widest_line_pt`'s full shaping pass, which is what makes this estimate safe: it is
/// only ever trusted when the face is provably uniform. Returns `None` (callers fall back to the
/// exact shaping measurement) when any token contains a tab: cosmic-text expands `\t` to an
/// 8-column tab stop, not a single cell, so a flat `char_count * advance` would understate the
/// line's width (an unsafe direction -- the block would overflow instead of shrinking).
pub fn estimate_code_natural_width_pt(font_system: &mut FontSystem, tokens: &[HighlightedToken], size: f32, font_family: &str) -> Option<f32> {
    let advance = monospace_advance_pt(font_system, font_family, size)?;
    if tokens.iter().any(|t| t.text.contains('\t')) {
        return None;
    }
    // Per line: the count up to and including the line's last non-whitespace character. Trailing
    // whitespace is excluded because shaping drops it (cosmic-text emits no advance-carrying
    // glyph for a line's trailing spaces), so counting it would overstate the line's width and
    // shrink blocks that actually fit.
    let mut widest = 0usize;
    let mut line = 0usize;
    let mut line_content = 0usize;
    for token in tokens {
        for c in token.text.chars() {
            if c == '\n' {
                widest = widest.max(line_content);
                line = 0;
                line_content = 0;
            } else {
                line += 1;
                if !c.is_whitespace() {
                    line_content = line;
                }
            }
        }
    }
    widest = widest.max(line_content);
    Some(widest as f32 * advance)
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
/// Disables the OpenType ligature features (`liga`/`dlig`/`clig`). Monospaced code faces ship
/// ligature glyphs (e.g. `fi`) whose advance is a *single* character cell, not two -- shaping
/// code with ligatures on therefore collapses `fi`/`ff`/`fl` pairs and shifts every column after
/// them left, breaking code alignment. Prose keeps ligatures (typographically desirable); code
/// turns them off so each character occupies exactly one cell and a line's width is exactly
/// `char_count * advance` (which is what makes `estimate_code_natural_width_pt` exact).
fn no_ligature_features() -> FontFeatures {
    FontFeatures {
        features: vec![
            Feature { tag: FeatureTag::STANDARD_LIGATURES, value: 0 },
            Feature { tag: FeatureTag::DISCRETIONARY_LIGATURES, value: 0 },
            Feature { tag: FeatureTag::CONTEXTUAL_LIGATURES, value: 0 },
        ],
    }
}

/// Shaping controls for `shape_rich_paragraph`. The only knob today is whether OpenType ligature
/// features stay on: prose keeps them (typographically desirable); code turns them off so each
/// character occupies exactly one cell -- see `no_ligature_features` for why that matters to column
/// alignment. The named constructors keep call sites self-documenting instead of passing a bare bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapingOptions {
    /// Keep OpenType ligature features (`liga`/`dlig`/`clig`) enabled while shaping.
    pub ligatures: bool,
}

impl ShapingOptions {
    /// Prose shaping: ligatures on.
    pub const PROSE: Self = Self { ligatures: true };
    /// Code shaping: ligatures off, so each glyph occupies exactly one character cell.
    pub const CODE: Self = Self { ligatures: false };
}

pub fn shape_rich_paragraph(font_system: &mut FontSystem, content: &[InlineNode], max_width_pt: f32, align: Align, options: ShapingOptions) -> Vec<ShapedRun> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::with_capacity(content.len());
    let mut rich_text_spans: Vec<(&str, Attrs)> = Vec::with_capacity(content.len());
    let mut offset = 0usize;
    for node in content {
        let mut attrs = Attrs::new()
            .family(resolve_family(font_system.db(), &node.style.font_family))
            .weight(if node.style.bold { Weight::BOLD } else { Weight::NORMAL })
            .style(if node.style.italic { Style::Italic } else { Style::Normal });
        if !options.ligatures {
            attrs = attrs.font_features(no_ligature_features());
        }
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

    // `spans` tile [0, full_text.len()) contiguously in document order (built above by a running
    // byte offset), and the glyph `global_start`s below arrive in that same forward order across
    // every run/glyph -- text flows top-to-bottom down the page, then left-to-right within each
    // visual line. So one cursor only ever moves FORWARD through the spans: O(glyphs + spans) total,
    // instead of the per-glyph linear scan this used to be (which was O(glyphs * spans)). The
    // regression guard keeps it correct even if a run's order were ever non-monotonic; in practice
    // that branch never fires.
    let mut span_cursor = 0usize;

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
            if global_start < spans[span_cursor].range.start {
                // Guard against a (never-observed) non-monotonic run order: reset to the start and
                // re-scan forward. Cold in practice; without it a regression would mis-resolve.
                span_cursor = 0;
            }
            while span_cursor + 1 < spans.len() && spans[span_cursor + 1].range.start <= global_start {
                span_cursor += 1;
            }
            let span_index = span_cursor;
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
