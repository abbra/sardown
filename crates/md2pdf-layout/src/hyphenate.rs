use hyphenation::{Hyphenator as _, Language, Load, Standard};

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
