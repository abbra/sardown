use crate::{shape_rich_paragraph, PathCommand, PositionedElement, ShapingOptions, StrokeStyle};
use cosmic_text::FontSystem;
use sardown_ast::InlineNode;

const MIN_COLUMN_WIDTH_PT: f32 = 40.0;

/// Added to every measured cell so a column sized exactly to its content doesn't wrap it over a
/// rounding difference (a hyphenated word like `ML-KEM-1024` would break at its hyphens).
const WRAP_SLACK_PT: f32 = 1.0;

/// Splits available width between columns in two passes:
///
/// 1. Every column gets a *floor* of at least its own longest single word's width (so a column
///    is never squeezed narrower than its own content can wrap without breaking a word), or the
///    fixed readable minimum if that's larger.
/// 2. Whatever width is left over is distributed proportionally to how much each column could
///    still use before it would stop wrapping at all (its longest unwrapped line minus its
///    floor). A column that's already satisfied by its floor (no wrapping needed) requests none
///    of the leftover.
///
/// The previous approach distributed the *entire* available width proportionally to each
/// column's longest unwrapped line. A column with a single short word (e.g. a "Key" or "File"
/// column) got squeezed down toward whatever tiny share that produced whenever another column
/// had long content, even though there was plenty of width to go around -- forcing the short
/// column's own words to wrap mid-word. Giving every column its word-wrap floor first fixes that
/// without giving up on distributing genuinely extra space to columns that can use it.
pub fn column_widths(
    headers: &[Vec<InlineNode>],
    rows: &[Vec<Vec<InlineNode>>],
    available_width_pt: f32,
    cell_padding_pt: f32,
    font_system: &mut FontSystem,
) -> Vec<f32> {
    let column_count = headers.len();
    let mut longest_word = vec![0.0f32; column_count];
    let mut longest_line = vec![0.0f32; column_count];

    let mut measure = |col: usize, cell: &[InlineNode]| {
        let m = measure_cell(font_system, cell, cell_padding_pt);
        longest_word[col] = longest_word[col].max(m.longest_word);
        longest_line[col] = longest_line[col].max(m.longest_line);
    };
    for (col, header) in headers.iter().enumerate() {
        measure(col, header);
    }
    for row in rows {
        for (col, cell) in row.iter().enumerate() {
            if col < column_count {
                measure(col, cell);
            }
        }
    }

    let floors: Vec<f32> = longest_word.iter().map(|&w| w.max(MIN_COLUMN_WIDTH_PT)).collect();
    let total_floor: f32 = floors.iter().sum();
    if total_floor <= 0.0 {
        return vec![available_width_pt / column_count as f32; column_count];
    }
    if total_floor >= available_width_pt {
        // Not even every column's word-wrap floor fits (e.g. one column's longest "word" is a
        // generic type path with no spaces, like `agirru_engine::GossipRuntime<IdpCrdt,`, wide
        // enough on its own to blow the budget). Plain proportional-by-floor still gives the
        // single neediest column the lion's share and squeezes every other column below its own
        // floor -- exactly the "narrow columns look bad" complaint this whole function exists to
        // avoid, just one level down. Max-min fair allocation instead gives every column its full
        // floor if there's room once smaller floors are satisfied first, letting only the
        // genuinely-oversized column(s) absorb the shortfall.
        return max_min_fair_allocation(&floors, available_width_pt);
    }

    let extra_wanted: Vec<f32> = longest_line.iter().zip(&floors).map(|(&line, &floor)| (line - floor).max(0.0)).collect();
    let total_extra_wanted: f32 = extra_wanted.iter().sum();
    let remaining = available_width_pt - total_floor;

    if total_extra_wanted <= 0.0 {
        // No column needs more than its floor to avoid wrapping (e.g. every column's content is
        // a single unbreakable token, so its longest line and longest word are the same). Split
        // the leftover proportionally to each column's own floor rather than evenly: an even
        // split would hand a column with a tiny floor (e.g. a short filename column) the same
        // raw leftover as a column whose floor is already huge (e.g. a long generic-type-like
        // token), ballooning the small column to an absurd width it has no content to fill.
        return floors.iter().map(|f| f + remaining * (f / total_floor)).collect();
    }

    floors.iter().zip(&extra_wanted).map(|(&floor, &wanted)| floor + remaining * (wanted / total_extra_wanted)).collect()
}

/// Standard max-min fair-share allocation: repeatedly give every column that can be fully
/// satisfied by an equal split of the *remaining* width its full request, freeing up its unused
/// share for the columns still waiting; once no more columns can be fully satisfied, split what's
/// left evenly across them. Unlike a single proportional pass, this guarantees every column gets
/// its full request whenever the total genuinely allows it, rather than one large request eating
/// into width a smaller request would have been fully satisfied by on its own.
fn max_min_fair_allocation(needs: &[f32], available: f32) -> Vec<f32> {
    let n = needs.len();
    let mut allocation = vec![0.0f32; n];
    let mut satisfied = vec![false; n];
    let mut remaining = available;
    let mut unsatisfied_count = n;

    while unsatisfied_count > 0 {
        let equal_share = remaining / unsatisfied_count as f32;
        let mut newly_satisfied = false;
        for i in 0..n {
            if !satisfied[i] && needs[i] <= equal_share {
                allocation[i] = needs[i];
                satisfied[i] = true;
                remaining -= needs[i];
                unsatisfied_count -= 1;
                newly_satisfied = true;
            }
        }
        if !newly_satisfied {
            for i in 0..n {
                if !satisfied[i] {
                    allocation[i] = equal_share;
                }
            }
            break;
        }
    }
    allocation
}

/// Both measurements `column_widths` needs for one cell, from a single unwrapped shaping pass.
/// The previous implementation shaped the whole cell once *and again once per word* (the word
/// floor came from isolated per-word shaping passes) -- a 100x4 table with ~5 words per cell
/// cost ~2,000 shaping passes before the table was ever placed. Deriving the per-word widths
/// from the same shaped glyph advances (words are whitespace-delimited runs of non-whitespace
/// clusters) keeps it at one pass per cell. Word widths measured in the cell's own text context
/// rather than in isolation can differ from the old isolated measurement by a fraction of a
/// kerning unit at most -- below the column floor's and the proportional distribution's own
/// smoothing, and below the visual-regression pixel threshold.
fn measure_cell(font_system: &mut FontSystem, cell: &[InlineNode], cell_padding_pt: f32) -> CellMeasure {
    // Shaped per span, so a cell mixing prose and inline code is measured in each span's own face.
    let runs = shape_rich_paragraph(font_system, cell, f32::MAX, cosmic_text::Align::Left, ShapingOptions::PROSE);
    let mut longest_line = 0.0f32;
    let mut longest_word = 0.0f32;
    let mut word_width = 0.0f32;
    for run in runs {
        if let PositionedElement::TextRun { glyphs, text, .. } = run.element {
            longest_line += glyphs.iter().map(|g| g.x_advance).sum::<f32>();
            for g in &glyphs {
                let cluster_is_whitespace = text[g.cluster.start..g.cluster.end].chars().next().is_some_and(char::is_whitespace);
                if cluster_is_whitespace {
                    longest_word = longest_word.max(word_width);
                    word_width = 0.0;
                } else {
                    word_width += g.x_advance;
                }
            }
        }
    }
    longest_word = longest_word.max(word_width);
    CellMeasure { longest_line: longest_line + cell_padding_pt + WRAP_SLACK_PT, longest_word: longest_word + cell_padding_pt + WRAP_SLACK_PT }
}

struct CellMeasure {
    longest_line: f32,
    longest_word: f32,
}

/// Everything one page segment of a table draws besides its text: row fills and rules. `rule_ys`
/// are the horizontal lines (top, under the header, under each row); `Grid` mode draws only the
/// one under the header, plus a vertical line per column boundary.
pub(crate) struct SegmentDecor {
    pub top_y: f32,
    pub bottom_y: f32,
    pub header_bottom_y: Option<f32>,
    pub rule_ys: Vec<f32>,
    pub fills: Vec<(f32, f32, [u8; 3])>,
}

/// The fill rectangles then the rules for one page segment of a table occupying
/// `[x, x + sum(widths)]`, fills first so they sit behind the text drawn before them in the list.
pub(crate) fn segment_elements(
    x: f32,
    widths: &[f32],
    decor: &SegmentDecor,
    style: &sardown_style::TableStyle,
) -> (Vec<PositionedElement>, Vec<PositionedElement>) {
    let right = x + widths.iter().sum::<f32>();
    let fills = decor
        .fills
        .iter()
        .map(|&(top, bottom, color)| PositionedElement::Path {
            points: vec![
                PathCommand::MoveTo(x, top),
                PathCommand::LineTo(right, top),
                PathCommand::LineTo(right, bottom),
                PathCommand::LineTo(x, bottom),
                PathCommand::Close,
            ],
            fill: Some(color),
            stroke: None,
        })
        .collect();
    let mut points = Vec::new();
    match style.border {
        sardown_style::TableBorder::None => {}
        sardown_style::TableBorder::Grid => {
            let mut cursor_x = x;
            for width in widths {
                cursor_x += width;
                points.push(PathCommand::MoveTo(cursor_x, decor.top_y));
                points.push(PathCommand::LineTo(cursor_x, decor.bottom_y));
            }
            if let Some(y) = decor.header_bottom_y {
                points.push(PathCommand::MoveTo(x, y));
                points.push(PathCommand::LineTo(right, y));
            }
        }
        sardown_style::TableBorder::Horizontal => {
            for &y in &decor.rule_ys {
                points.push(PathCommand::MoveTo(x, y));
                points.push(PathCommand::LineTo(right, y));
            }
        }
    }
    let rules = if points.is_empty() {
        Vec::new()
    } else {
        vec![PositionedElement::Path { points, fill: None, stroke: Some(StrokeStyle { color: style.border_color.0, width: style.border_width_pt }) }]
    };
    (fills, rules)
}
