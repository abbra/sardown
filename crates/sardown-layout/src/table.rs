use crate::{shape_paragraph, PathCommand, PositionedElement, StrokeStyle};
use cosmic_text::FontSystem;
use sardown_ast::InlineNode;

const MIN_COLUMN_WIDTH_PT: f32 = 40.0;

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
pub fn column_widths(headers: &[Vec<InlineNode>], rows: &[Vec<Vec<InlineNode>>], available_width_pt: f32, font_system: &mut FontSystem) -> Vec<f32> {
    let column_count = headers.len();
    let mut longest_word = vec![0.0f32; column_count];
    let mut longest_line = vec![0.0f32; column_count];

    let mut measure = |col: usize, cell: &[InlineNode], font_system: &mut FontSystem| {
        longest_word[col] = longest_word[col].max(measure_longest_word_width(font_system, cell));
        longest_line[col] = longest_line[col].max(measure_unwrapped_width(font_system, cell));
    };
    for (col, header) in headers.iter().enumerate() {
        measure(col, header, font_system);
    }
    for row in rows {
        for (col, cell) in row.iter().enumerate() {
            if col < column_count {
                measure(col, cell, font_system);
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

fn measure_unwrapped_width(font_system: &mut FontSystem, cell: &[InlineNode]) -> f32 {
    let elements = shape_paragraph(font_system, cell, f32::MAX);
    elements
        .into_iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { glyphs, .. } => Some(glyphs.iter().map(|g| g.x_advance).sum::<f32>()),
            _ => None,
        })
        .fold(0.0, f32::max)
        + 12.0 // cell padding
}

fn measure_longest_word_width(font_system: &mut FontSystem, cell: &[InlineNode]) -> f32 {
    let Some(style) = cell.first().map(|n| n.style.clone()) else {
        return 0.0;
    };
    let full_text: String = cell.iter().map(|n| n.text.as_str()).collect();
    full_text
        .split_whitespace()
        .map(|word| {
            let node = InlineNode { text: word.to_string(), style: style.clone(), link_target: None };
            measure_unwrapped_width(font_system, std::slice::from_ref(&node))
        })
        .fold(0.0, f32::max)
}

/// Grid line Path for a table occupying [x, x + total_width] x [top_y, bottom_y], with one
/// vertical line per column boundary and, if this page segment includes the header row, one
/// horizontal line under it. `header_bottom_y` is `None` for a page a table continues onto
/// after a row-level page break, since the header is not repeated there.
pub(crate) fn grid_path(x: f32, top_y: f32, bottom_y: f32, header_bottom_y: Option<f32>, widths: &[f32]) -> PositionedElement {
    let mut points = vec![PathCommand::MoveTo(x, top_y)];
    let mut cursor_x = x;
    for width in widths {
        cursor_x += width;
        points.push(PathCommand::MoveTo(cursor_x, top_y));
        points.push(PathCommand::LineTo(cursor_x, bottom_y));
    }
    if let Some(header_bottom_y) = header_bottom_y {
        points.push(PathCommand::MoveTo(x, header_bottom_y));
        points.push(PathCommand::LineTo(cursor_x, header_bottom_y));
    }
    PositionedElement::Path { points, fill: None, stroke: Some(StrokeStyle { color: [180, 180, 180], width: 0.75 }) }
}
