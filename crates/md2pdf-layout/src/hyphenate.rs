use crate::{shape_paragraph, PositionedElement};
use cosmic_text::FontSystem;
use hyphenation::{Hyphenator as _, Language, Load, Standard};
use md2pdf_ast::InlineNode;

/// Wraps a loaded `hyphenation` dictionary for one language.
pub struct Hyphenator {
    dictionary: Standard,
}

impl Hyphenator {
    /// Loads the dictionary for `language_code` (e.g. "en-us" -- see `hyphenation::Language::code`
    /// for the full set this project embeds via `embed_all`). Returns `None` with a warning on
    /// stderr if the code isn't recognized or the embedded dictionary can't be loaded, matching
    /// this project's graceful-degradation convention (unresolvable font family, unresolvable
    /// numbering-reset heading): hyphenation is disabled for the render, not the whole render.
    pub fn load(language_code: &str) -> Option<Hyphenator> {
        let language = match Language::try_from_code(language_code) {
            Some(language) => language,
            None => {
                eprintln!("warning: unknown hyphenation language {language_code:?}; hyphenation disabled for this render");
                return None;
            }
        };
        match Standard::from_embedded(language) {
            Ok(dictionary) => Some(Hyphenator { dictionary }),
            Err(e) => {
                eprintln!("warning: failed to load hyphenation dictionary for {language_code:?}: {e}; hyphenation disabled for this render");
                None
            }
        }
    }

    /// Byte offsets within `word` where a hyphen may be inserted (`word[..i]` + "-" + `word[i..]`),
    /// honoring the dictionary's own minimum-characters-before/after-break rule. Case-insensitive;
    /// offsets are already realigned to `word`'s own bytes by the `hyphenation` crate itself.
    pub fn candidate_breaks(&self, word: &str) -> Vec<usize> {
        self.dictionary.hyphenate(word).breaks
    }
}

fn measure_text_width(font_system: &mut FontSystem, style: &md2pdf_ast::TextStyle, text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let node = InlineNode { text: text.to_string(), style: style.clone(), link_target: None };
    let elements = shape_paragraph(font_system, std::slice::from_ref(&node), f32::MAX);
    elements
        .into_iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { glyphs, .. } => Some(glyphs.iter().map(|g| g.x_advance).sum::<f32>()),
            _ => None,
        })
        .sum()
}

/// A hyphenation candidate: every character alphabetic -- no digits, punctuation, or apostrophes.
/// A deliberate v1 scope cut (see the design doc's "Word selection") to avoid a large class of
/// punctuation-reconstruction edge cases for a typographic-polish feature.
fn is_hyphenation_candidate(word: &str) -> bool {
    !word.is_empty() && word.chars().all(char::is_alphabetic)
}

/// Inserts a literal "-\n" at each point a word needed to split to fit `max_width_pt`, simulating
/// just enough of a greedy word-wrap to know where a fragment would fit. `content` is otherwise
/// left untouched (most paragraphs get zero insertions); the result is handed to the existing,
/// unmodified `shape_rich_paragraph`, which already treats "\n" as a hard break and performs all
/// real shaping/justification/wrapping itself -- see the design doc for why a soft hyphen can't
/// be used instead.
///
/// A word straddling two `InlineNode`s (a style-span boundary) is never hyphenated; each node's
/// text is tokenized independently, which very rarely (only when inline formatting lands mid-word,
/// essentially never in real prose) means the running width estimate for the word immediately
/// following a straddling one is a node-boundary's worth off -- an accepted approximation, since
/// hyphenation is only ever opportunistic, never a forced break beyond what was already measured
/// to fit.
pub fn insert_hyphenation_breaks(content: &[InlineNode], hyphenator: &Hyphenator, max_width_pt: f32, font_system: &mut FontSystem) -> Vec<InlineNode> {
    let mut result = content.to_vec();
    let mut line_width = 0.0f32;
    let mut line_has_content = false;
    let node_count = result.len();

    for node_idx in 0..node_count {
        let style = result[node_idx].style.clone();
        let space_width = measure_text_width(font_system, &style, " ");
        let hyphen_width = measure_text_width(font_system, &style, "-");
        let text = std::mem::take(&mut result[node_idx].text);
        // The word at the very end of this node's text (if any) continues into the next node
        // when there IS a next node, this node's text doesn't end in whitespace, and the next
        // node's text doesn't start with whitespace either -- such a word must never be chosen
        // for hyphenation (see the design doc), only its width tracking is approximated.
        let continues_into_next_node = node_idx + 1 < node_count
            && !text.is_empty()
            && !text.ends_with(|c: char| c.is_whitespace())
            && result[node_idx + 1].text.chars().next().is_some_and(|c| !c.is_whitespace());

        let mut new_text = String::with_capacity(text.len());
        let mut cursor = 0usize;

        loop {
            let rest = &text[cursor..];
            let word_start = rest.find(|c: char| !c.is_whitespace()).unwrap_or(rest.len());
            new_text.push_str(&rest[..word_start]);
            if word_start == rest.len() {
                break;
            }
            let after_ws = &rest[word_start..];
            let word_len = after_ws.find(char::is_whitespace).unwrap_or(after_ws.len());
            let word = &after_ws[..word_len];
            let word_abs_start = cursor + word_start;
            let is_last_word_in_node = word_abs_start + word_len == text.len();
            let straddles_node_boundary = is_last_word_in_node && continues_into_next_node;
            let word_width = measure_text_width(font_system, &style, word);
            let prefix_width = if line_has_content { space_width } else { 0.0 };

            if line_width + prefix_width + word_width <= max_width_pt {
                new_text.push_str(word);
                line_width += prefix_width + word_width;
                line_has_content = true;
                cursor = word_abs_start + word_len;
                continue;
            }

            let available = if line_has_content { max_width_pt - line_width - space_width } else { max_width_pt };
            let split = if !straddles_node_boundary && is_hyphenation_candidate(word) {
                hyphenator.candidate_breaks(word).into_iter().rev().find(|&i| measure_text_width(font_system, &style, &word[..i]) + hyphen_width <= available)
            } else {
                None
            };

            match split {
                Some(i) => {
                    new_text.push_str(&word[..i]);
                    new_text.push_str("-\n");
                    cursor = word_abs_start + i;
                    line_width = 0.0;
                    line_has_content = false;
                }
                None => {
                    new_text.push_str(word);
                    line_width = word_width;
                    line_has_content = true;
                    cursor = word_abs_start + word_len;
                }
            }
        }
        result[node_idx].text = new_text;
    }
    result
}
