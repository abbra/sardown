use crate::{shape_paragraph, PositionedElement};
use cosmic_text::FontSystem;
use hyphenation::{Hyphenator as _, Language, Load, Standard};
use sardown_ast::InlineNode;

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

/// One word shaped exactly once (unwrapped), with per-glyph advance data kept so that the
/// width of any `word[..offset]` prefix can be derived by lookup instead of a second shaping
/// pass. Shaping a word per candidate hyphenation prefix -- the previous approach -- turned a
/// single long word with N dictionary break points into N+1 full cosmic-text shaping passes;
/// here the whole word costs exactly one pass regardless of how many break candidates exist.
///
/// That one pass is itself cached: prose reuses the same few hundred/thousand words over and
/// over, and the wrap simulation below shapes *every* word of *every* paragraph (to track
/// running line widths), so an uncached path paid a full cosmic-text `Buffer` construction +
/// shaping pass per word occurrence -- measured at ~3x layout time on a 2,400-paragraph prose
/// corpus with hyphenation enabled. Results are memoized per style (family + size + bold/
/// italic -- the only style fields that affect advance widths; color/strikethrough never do)
/// in the thread-local `WORD_SHAPING_CACHE`, alongside this crate's other shaping caches in
/// `shaping_cache.rs`. The inner map is keyed by `str` so cache hits allocate nothing; only
/// misses pay the boxed-word key. Cache scope is one font database, not the thread -- see
/// `shaping_cache.rs` for the two-layer invalidation that keeps a second document rendered
/// through a different `FontSystem` from inheriting this document's word widths.
#[derive(Clone)]
pub(crate) struct ShapedWord {
    /// `(cluster_start, cumulative advance up to and including the glyph starting there)`, in
    /// cluster order (which is text order for an unwrapped single word).
    advances: Vec<(usize, f32)>,
    total: f32,
}

fn shape_word(font_system: &mut FontSystem, style: &sardown_ast::TextStyle, word: &str) -> ShapedWord {
    crate::shaping_cache::note_font_system(font_system);
    let key = crate::shaping_cache::WordStyleKey::of(style);
    if let Some(hit) = crate::shaping_cache::word_cache_lookup(&key, word) {
        return hit;
    }
    let node = InlineNode { text: word.to_string(), style: style.clone(), link_target: None };
    let elements = shape_paragraph(font_system, std::slice::from_ref(&node), f32::MAX);
    let mut advances = Vec::new();
    let mut total = 0.0f32;
    for element in elements {
        if let PositionedElement::TextRun { glyphs, .. } = element {
            for g in &glyphs {
                total += g.x_advance;
                advances.push((g.cluster.start, total));
            }
        }
    }
    let shaped = ShapedWord { advances, total };
    crate::shaping_cache::word_cache_insert(key, word, shaped.clone());
    shaped
}

impl ShapedWord {
    fn width(&self) -> f32 {
        self.total
    }

    /// The width of `word[..offset]`: the sum of the advances of every glyph whose cluster
    /// starts before `offset`. Hyphenation break offsets always land on character boundaries
    /// the dictionary permits, so a cluster never straddles `offset` mid-grapheme.
    fn prefix_width(&self, offset: usize) -> f32 {
        // First entry whose cluster start is >= offset; everything before it is inside the prefix.
        let idx = self.advances.partition_point(|(start, _)| *start < offset);
        self.advances.get(idx - 1).map(|(_, w)| *w).unwrap_or(0.0)
    }
}

/// A hyphenation candidate: every character alphabetic -- no digits, punctuation, or apostrophes.
/// A deliberate v1 scope cut (see the design doc's "Word selection") to avoid a large class of
/// punctuation-reconstruction edge cases for a typographic-polish feature.
fn is_hyphenation_candidate(word: &str) -> bool {
    !word.is_empty() && word.chars().all(char::is_alphabetic)
}

/// Inserts a literal "-" at each point a word needed to split to fit `max_width_pt`, simulating
/// just enough of a greedy word-wrap to know where a fragment would fit. `content` is otherwise
/// left untouched (most paragraphs get zero insertions); the result is handed to the existing,
/// unmodified `shape_rich_paragraph` -- see the design doc for why a soft hyphen can't be used
/// instead.
///
/// Deliberately *not* a forced/mandatory break (no "\n", no U+2028 LINE SEPARATOR either -- both
/// were tried and both exempt their line from justification, since cosmic-text, matching real
/// typesetting convention, never justifies a line ending in *any* mandatory break, not just a
/// paragraph-ending one). A bare hyphen-minus is already an *allowed* break-after point under
/// UAX #14 (`unicode-linebreak`, which cosmic-text's own wrapping already uses, classifies "-" as
/// class BA) -- exactly like the hyphen in a naturally-typed word such as "well-known" already
/// wraps correctly today. Inserting only the hyphen and leaving the actual wrap decision to
/// cosmic-text's own normal, real-metrics-based line-breaking means a hyphenated line is wrapped
/// via the exact same mechanism as any other line in the paragraph, so it stays eligible for
/// justification like everything else.
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
        // Measured once per node (not per word): both are single-character and never change
        // within the node's own style.
        let space_width = shape_word(font_system, &style, " ").width();
        let hyphen_width = shape_word(font_system, &style, "-").width();
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
            let shaped_word = shape_word(font_system, &style, word);
            let word_width = shaped_word.width();
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
                hyphenator.candidate_breaks(word).into_iter().rev().find(|&i| shaped_word.prefix_width(i) + hyphen_width <= available)
            } else {
                None
            };

            match split {
                Some(i) => {
                    new_text.push_str(&word[..i]);
                    new_text.push('-');
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
