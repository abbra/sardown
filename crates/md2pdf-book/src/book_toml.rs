use std::path::{Path, PathBuf};

/// Reads `book_root/book.toml` for `[book].src` (mdBook's own source-directory override),
/// defaulting to `"src"` if the key, the `[book]` table, the file itself, or the file's TOML
/// syntax is missing/invalid -- an mdBook project without a `book.toml` at all is valid (mdBook
/// itself accepts this), so absence is not an error here.
pub fn resolve_src_dir(book_root: &Path) -> PathBuf {
    const DEFAULT_SRC: &str = "src";
    // `toml::Value: FromStr` parses a single TOML *value* (e.g. an inline table or array
    // literal), not a full document with `[section]` headers -- confirmed by observing it reject
    // a real `book.toml` with "unexpected content, expected nothing". `toml::Table: FromStr` is
    // the document-level parser and is what the crate's own docs recommend for this.
    let src = std::fs::read_to_string(book_root.join("book.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|table| table.get("book").and_then(|b| b.get("src")).and_then(|s| s.as_str()).map(str::to_string))
        .unwrap_or_else(|| DEFAULT_SRC.to_string());
    book_root.join(src)
}
