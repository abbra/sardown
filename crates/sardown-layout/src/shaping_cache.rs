//! Thread-local memoization for text shaping, scoped to one font database.
//!
//! Three caches live here:
//!
//! - `WORD_SHAPING_CACHE`: per-style glyph advances for single words, backing hyphenation's
//!   wrap simulation (see `hyphenate.rs` for why it exists).
//! - `MONOSPACE_ADVANCE_CACHE`: the "is this face monospaced, and what is one cell's advance"
//!   probe behind `shape::monospace_advance_pt`.
//! - `FAMILY_KNOWN_CACHE`: whether a literal stylesheet family name resolves against the
//!   loaded faces, behind `shape::resolve_family`.
//!
//! Every one of them answers a question whose result depends on *which faces are loaded in
//! the active [`cosmic_text::FontSystem`]'s database* -- not just on the style key it is
//! indexed by. A thread that renders two documents through two different `FontSystem`s must
//! therefore never let the second observe the first's entries: identical style keys would
//! silently return the first document's advances, corrupting wrapping and hyphenation.
//! Keying entries by the database's address is not sufficient on its own: sequentially
//! dropped and re-created systems routinely reuse the same heap allocation, and an address
//! comparison cannot tell that ABA case apart. Scoping is therefore two layers deep:
//!
//! 1. [`note_font_system`] runs before every cache-touching path (both shaping entry points,
//!    the monospace probe, and hyphenation's `shape_word`). It compares the current
//!    database's address against the last one seen on this thread and clears every cache on
//!    change. Any switch between simultaneously distinct font systems is caught here,
//!    wherever the systems came from.
//! 2. [`reset_shaping_caches`] unconditionally clears everything. Document-level entry
//!    points (`layout_impl`, `layout_with_header_footer`, sardown-slides'
//!    `render_slide_deck`) call it so a freshly constructed `FontSystem` can never inherit
//!    entries even when its database lands on a recycled address.
//!
//! Embedders that drive their own `FontSystem`s straight through the low-level helpers
//! (`shape_paragraph`, `shape_rich_paragraph`, `measure_widest_line_pt`, ...) rather than a
//! document entry point should call [`reset_shaping_caches`] themselves when switching to a
//! system with different loaded fonts.
//!
//! Deliberately *not* reset: `layout_with_assets`. It exists precisely so the slide
//! auto-shrink loop can re-layout the same slide many times per second against one unchanging
//! `FontSystem`; wiping there would discard warm word entries on every shrink iteration.
//! Within-document correctness needs no help from these caches' invalidation anyway -- one
//! layout pass sees exactly one font system.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use cosmic_text::FontSystem;
use sardown_ast::TextStyle;

use crate::hyphenate::ShapedWord;

/// Upper bound on distinct words remembered per style, so a long-lived embedding process can't
/// grow the cache without limit across arbitrarily many documents. A single render's vocabulary
/// sits orders of magnitude below this; hitting it just forgets that style's words and starts
/// over.
const WORD_CACHE_MAX_ENTRIES: usize = 100_000;

/// The style fields `shape_word`'s output actually depends on.
#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) struct WordStyleKey {
    family: Arc<str>,
    size_bits: u32,
    bold: bool,
    italic: bool,
}

impl WordStyleKey {
    pub(crate) fn of(style: &TextStyle) -> Self {
        Self { family: Arc::clone(&style.font_family), size_bits: style.size.to_bits(), bold: style.bold, italic: style.italic }
    }
}

thread_local! {
    /// Address of the font database whose entries the three caches below hold, so a switch
    /// between different systems on one thread can invalidate them ([`note_font_system`]).
    static LAST_FONT_DB_ADDR: Cell<usize> = const { Cell::new(0) };
    static WORD_SHAPING_CACHE: RefCell<HashMap<WordStyleKey, HashMap<Box<str>, ShapedWord>>> = RefCell::new(HashMap::new());
    static MONOSPACE_ADVANCE_CACHE: RefCell<HashMap<(String, u32), Option<f32>>> = RefCell::new(HashMap::new());
    static FAMILY_KNOWN_CACHE: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

/// Identity of the database inside `font_system`. The `Database` lives by value inside the
/// `FontSystem` (cosmic-text 0.19), so its address is stable exactly as long as that system
/// is -- which is all this comparison needs: equal addresses while both systems are alive is
/// impossible, and the recycled-address case after a system dies is layer 2's job.
fn font_db_addr(font_system: &FontSystem) -> usize {
    std::ptr::from_ref(font_system.db()) as usize
}

/// Layer-1 scope check: forget every cache unless this is the same font system -- strictly,
/// the same database address -- as the last shaping call on this thread. One integer compare
/// on the warm path; call it at the top of anything that reads or writes the caches below.
pub(crate) fn note_font_system(font_system: &FontSystem) {
    let addr = font_db_addr(font_system);
    if LAST_FONT_DB_ADDR.with(Cell::get) != addr {
        clear_all();
        LAST_FONT_DB_ADDR.with(|slot| slot.set(addr));
    }
}

/// Layer-2 hard reset: drop every memoized entry now. Public because embedders juggling
/// several `FontSystem`s through the low-level shaping helpers need the same guarantee the
/// crate's own document entry points give themselves.
pub fn reset_shaping_caches() {
    clear_all();
}

fn clear_all() {
    WORD_SHAPING_CACHE.with(|c| c.borrow_mut().clear());
    MONOSPACE_ADVANCE_CACHE.with(|c| c.borrow_mut().clear());
    FAMILY_KNOWN_CACHE.with(|c| c.borrow_mut().clear());
}

/// `shape_word`'s memoized lookup. The caller has already run [`note_font_system`], so any
/// entries found here belong to the active system.
pub(crate) fn word_cache_lookup(key: &WordStyleKey, word: &str) -> Option<ShapedWord> {
    WORD_SHAPING_CACHE.with(|c| c.borrow().get(key).and_then(|inner| inner.get(word)).cloned())
}

/// `shape_word`'s memoized store. Inner map is keyed by `str` so hits allocate nothing; only
/// misses pay the boxed-word key. Hitting the size bound forgets that style's words rather
/// than growing without limit.
pub(crate) fn word_cache_insert(key: WordStyleKey, word: &str, shaped: ShapedWord) {
    WORD_SHAPING_CACHE.with(|c| {
        let mut outer = c.borrow_mut();
        let inner = outer.entry(key).or_default();
        if inner.len() >= WORD_CACHE_MAX_ENTRIES {
            inner.clear();
        }
        inner.insert(Box::from(word), shaped);
    });
}

/// `monospace_advance_pt`'s get-or-shape: returns the memoized probe verdict for
/// `(family, size)`, or runs `miss` once and remembers it.
pub(crate) fn monospace_cached(key: &(String, u32), miss: impl FnOnce() -> Option<f32>) -> Option<f32> {
    if let Some(hit) = MONOSPACE_ADVANCE_CACHE.with(|c| c.borrow().get(key).copied()) {
        return hit;
    }
    let result = miss();
    MONOSPACE_ADVANCE_CACHE.with(|c| c.borrow_mut().insert(key.clone(), result));
    result
}

/// `resolve_family`'s get-or-scan for a literal family name: returns whether the name is
/// known to the active database, or runs `miss` once (which owns the unknown-family warning,
/// so it stays first-per-name-per-scope like before).
pub(crate) fn family_known_cached(name: &str, miss: impl FnOnce() -> bool) -> bool {
    if let Some(known) = FAMILY_KNOWN_CACHE.with(|c| c.borrow().get(name).copied()) {
        return known;
    }
    let known = miss();
    FAMILY_KNOWN_CACHE.with(|c| c.borrow_mut().insert(name.to_owned(), known));
    known
}
