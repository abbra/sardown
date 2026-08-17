use crate::AnchorTable;
use md2pdf_style::{NumberingFormat, PageNumbering};

const ROMAN_NUMERALS: [(u32, &str); 13] =
    [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];

fn to_roman(mut n: u32) -> String {
    let mut result = String::new();
    for &(value, numeral) in &ROMAN_NUMERALS {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }
    result
}

/// Formats `n` per `format`. Roman numerals have no representation for 0 and become unwieldy
/// past the conventional range of 1-3999, so both fall back to the plain arabic form -- silently
/// for 0 (an unusual but harmless `start_at = 0` choice), with a warning above 3999.
pub fn format_page_number(n: u32, format: NumberingFormat) -> String {
    match format {
        NumberingFormat::Arabic => n.to_string(),
        NumberingFormat::RomanLower | NumberingFormat::RomanUpper => {
            if n == 0 {
                return n.to_string();
            }
            if n > 3999 {
                eprintln!(
                    "warning: page number {n} exceeds the conventional roman numeral range (1-3999); \
                     falling back to arabic for this value"
                );
                return n.to_string();
            }
            let roman = to_roman(n);
            match format {
                NumberingFormat::RomanLower => roman.to_lowercase(),
                _ => roman,
            }
        }
    }
}

/// A numbering config active from some physical page onward: either the document's own base
/// `[page.numbering]` (`from_page: 0`), or a reset's config from its heading's resolved page.
pub(crate) struct NumberingSegment {
    from_page: usize,
    format: NumberingFormat,
    start_at: u32,
}

/// Resolves `numbering` and every configured reset into a page-ascending list of segments (always
/// at least the base segment), skipping -- with a warning -- any reset whose `at_heading` doesn't
/// match a real heading id. There's no validation-time way to know which heading ids will exist,
/// so a typo'd `at_heading` would otherwise silently apply the base numbering for the rest of the
/// document instead of signaling the mistake.
pub(crate) fn resolve_numbering_segments(numbering: &PageNumbering, anchors: &AnchorTable) -> Vec<NumberingSegment> {
    let mut segments = vec![NumberingSegment { from_page: 0, format: numbering.format, start_at: numbering.start_at }];
    for reset in &numbering.resets {
        match anchors.get(&reset.at_heading) {
            Some(anchor) => segments.push(NumberingSegment { from_page: anchor.page, format: reset.format, start_at: reset.start_at }),
            None => eprintln!("warning: [page.numbering] reset references unknown heading id {:?} -- ignoring this reset", reset.at_heading),
        }
    }
    segments.sort_by_key(|s| s.from_page);
    segments
}

/// The formatted page-number text for `page_index` (0-indexed physical page), per whichever
/// segment is active there -- the last segment (by page order) whose `from_page <= page_index`.
pub(crate) fn display_number_for_page(page_index: usize, segments: &[NumberingSegment]) -> String {
    let segment = segments.iter().rev().find(|s| s.from_page <= page_index).expect("resolve_numbering_segments always includes a from_page: 0 base segment");
    let offset_into_segment = (page_index - segment.from_page) as u32;
    format_page_number(segment.start_at + offset_into_segment, segment.format)
}
