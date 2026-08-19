use base64::{engine::general_purpose::STANDARD, Engine as _};
use sardown_ast::{BlockNode, ImageSource};
use sardown_enrich::{CompiledDiagram, DiagramTable};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Resolves `path` against `base_dir` and rejects the result unless it stays within `base_dir`.
///
/// `path` comes directly from Markdown authored by whoever wrote the document being rendered —
/// without this check, `![x](../../../etc/passwd)` (relative traversal) or `![x](/etc/passwd)`
/// (an absolute path, which `Path::join` uses verbatim, discarding `base_dir` entirely) would let
/// a document embed arbitrary local files into the output PDF. Canonicalizing resolves `..` and
/// symlinks via the OS, so the `starts_with` check can't be fooled by a symlink that lexically
/// looks contained but points outside `base_dir`.
/// `resolve_within_base`, with the base directory's own canonicalization hoisted out: the
/// canonical base is computed ONCE per document (see `decode_images` / `collect_svg_diagrams`)
/// and passed in, instead of re-canonicalizing `base_dir` -- a real syscall-heavy path
/// resolution -- for every single image in the document.
fn resolve_within_base(base_dir: &Path, canonical_base: &Path, path: &Path) -> Result<PathBuf, String> {
    let candidate = base_dir.join(path);
    let canonical_candidate = candidate.canonicalize().map_err(|e| format!("cannot resolve path: {e}"))?;
    if canonical_candidate.starts_with(canonical_base) {
        Ok(canonical_candidate)
    } else {
        Err(format!("path escapes base directory {}", canonical_base.display()))
    }
}

#[derive(Clone)]
pub struct DecodedImage {
    /// `Arc`-backed so a table of decoded images can be cloned cheaply (the slide auto-shrink
    /// loop and the per-slide `LayoutOutput`s share one deck-wide table) without re-copying
    /// megapixel pixel buffers; the one real copy happens where krilla's `Image::from_rgba8`
    /// API demands an owned `Vec<u8>`.
    pub rgba8: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

pub type ImageTable = HashMap<String, DecodedImage>;

pub fn decode_images(ast: &[BlockNode], base_dir: &Path) -> ImageTable {
    let mut table = HashMap::new();
    // A `data:` URI needs no filesystem access, so it can always be decoded even when `base_dir`
    // can't be resolved. Only filesystem-backed (`Embedded`) images require a canonical base; if
    // it can't be resolved we skip just those and still decode every embedded data URI.
    let canonical_base = match base_dir.canonicalize() {
        Ok(cb) => Some(cb),
        Err(e) => {
            eprintln!("warning: cannot resolve base directory {}: {e}; filesystem images will not be loaded", base_dir.display());
            None
        }
    };
    collect(ast, base_dir, canonical_base.as_deref(), &mut table);
    table
}

fn is_svg_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

/// The declared MIME type of a `data:` URI (e.g. `"image/png"`), or a generic fallback if the
/// header doesn't parse -- used only for warning messages, never the raw base64 payload, which
/// could be megabytes of noise dumped to stderr for one broken image.
fn data_uri_label(uri: &str) -> &str {
    uri.strip_prefix("data:").and_then(|rest| rest.split([',', ';']).next()).filter(|mime| !mime.is_empty()).unwrap_or("data URI")
}

fn is_svg_data_uri(uri: &str) -> bool {
    data_uri_label(uri).eq_ignore_ascii_case("image/svg+xml")
}

/// Decodes a `data:<mime>;base64,<data>` URI into its raw bytes. Only the base64-encoded form is
/// supported -- Markdown-embedded images are essentially always produced this way by editors and
/// export tools; the rarer percent-encoded form is left unsupported rather than silently
/// mis-decoded. Tolerates embedded whitespace/newlines in the base64 payload, since some tools
/// wrap it at a fixed column width.
fn decode_data_uri(uri: &str) -> Result<Vec<u8>, String> {
    let rest = uri.strip_prefix("data:").ok_or("not a data: URI")?;
    let (header, data) = rest.split_once(',').ok_or("data: URI has no ',' separator")?;
    if !header.ends_with(";base64") {
        return Err("only base64-encoded data: URIs are supported".to_string());
    }
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(&cleaned).map_err(|e| format!("invalid base64 data: {e}"))
}

fn collect(ast: &[BlockNode], base_dir: &Path, canonical_base: Option<&Path>, table: &mut ImageTable) {
    for block in ast {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } => {
                // .svg files are collected separately by collect_svg_diagrams -- the `image`
                // crate has no SVG decoder, so leaving this arm to try would just produce a
                // spurious "failed to decode" warning for every embedded SVG.
                if is_svg_path(path) {
                    continue;
                }
                let key = path.to_string_lossy().to_string();
                if table.contains_key(&key) {
                    continue;
                }
                let Some(canonical_base) = canonical_base else {
                    continue; // base_dir unresolvable: filesystem images can't be loaded
                };
                match resolve_within_base(base_dir, canonical_base, path) {
                    Ok(resolved) => match image::open(&resolved) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            table.insert(key, DecodedImage { width: rgba.width(), height: rgba.height(), rgba8: Arc::new(rgba.into_raw()) });
                        }
                        Err(e) => eprintln!("warning: failed to decode image {key}: {e}"),
                    },
                    Err(e) => eprintln!("warning: refusing to load image {key}: {e}"),
                }
            }
            BlockNode::Image { source: ImageSource::DataUri(uri), .. } => {
                // SVG data URIs are collected separately by collect_svg_diagrams, same reasoning
                // as is_svg_path above: the `image` crate has no SVG decoder.
                if is_svg_data_uri(uri) {
                    continue;
                }
                if table.contains_key(uri) {
                    continue;
                }
                match decode_data_uri(uri) {
                    Ok(bytes) => match image::load_from_memory(&bytes) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            table.insert(uri.clone(), DecodedImage { width: rgba.width(), height: rgba.height(), rgba8: Arc::new(rgba.into_raw()) });
                        }
                        Err(e) => eprintln!("warning: failed to decode embedded {} image: {e}", data_uri_label(uri)),
                    },
                    Err(e) => eprintln!("warning: refusing to load embedded {} image: {e}", data_uri_label(uri)),
                }
            }
            BlockNode::Image { source: ImageSource::External(url), .. } => {
                eprintln!("warning: skipping external image (not fetched): {url}");
            }
            BlockNode::Blockquote { content } => collect(content, base_dir, canonical_base, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, base_dir, canonical_base, table);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect(column, base_dir, canonical_base, table);
                }
            }
            _ => {}
        }
    }
}

/// Collects embedded `.svg` image files into a `DiagramTable`, keyed the same way
/// `decode_images` keys raster images -- `render_block`'s `BlockNode::Image` arm checks the
/// raster table first, then this one, so whichever table actually has an entry for a given path
/// determines whether it renders as a `RasterImage` or a `VectorGraphic`. Reuses the exact same
/// `resolve_within_base` security boundary as raster images: an SVG file is still Markdown-author-
/// controlled input, and needs the same protection against path traversal.
pub fn collect_svg_diagrams(ast: &[BlockNode], base_dir: &Path) -> DiagramTable {
    let mut table = HashMap::new();
    // A `data:` URI needs no filesystem access, so it can always be collected even when `base_dir`
    // can't be resolved. Only filesystem-backed (`Embedded`) SVGs require a canonical base; if it
    // can't be resolved we skip just those and still collect every embedded SVG data URI.
    let canonical_base = match base_dir.canonicalize() {
        Ok(cb) => Some(cb),
        Err(e) => {
            eprintln!("warning: cannot resolve base directory {}: {e}; filesystem SVG images will not be loaded", base_dir.display());
            None
        }
    };
    collect_svgs(ast, base_dir, canonical_base.as_deref(), &mut table);
    table
}

fn collect_svgs(ast: &[BlockNode], base_dir: &Path, canonical_base: Option<&Path>, table: &mut DiagramTable) {
    for block in ast {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } if is_svg_path(path) => {
                let key = path.to_string_lossy().to_string();
                if table.contains_key(&key) {
                    continue;
                }
                let Some(canonical_base) = canonical_base else {
                    continue; // base_dir unresolvable: filesystem SVGs can't be loaded
                };
                match resolve_within_base(base_dir, canonical_base, path) {
                    Ok(resolved) => match std::fs::read_to_string(&resolved) {
                        Ok(svg) => match usvg::Tree::from_str(&svg, &usvg::Options::default()) {
                            Ok(tree) => {
                                let size = tree.size();
                                table.insert(key, CompiledDiagram { svg, width: size.width(), height: size.height() });
                            }
                            Err(e) => eprintln!("warning: failed to parse SVG image {key}: {e}"),
                        },
                        Err(e) => eprintln!("warning: failed to read SVG image {key}: {e}"),
                    },
                    Err(e) => eprintln!("warning: refusing to load image {key}: {e}"),
                }
            }
            BlockNode::Image { source: ImageSource::DataUri(uri), .. } if is_svg_data_uri(uri) => {
                if table.contains_key(uri) {
                    continue;
                }
                match decode_data_uri(uri) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(svg) => match usvg::Tree::from_str(&svg, &usvg::Options::default()) {
                            Ok(tree) => {
                                let size = tree.size();
                                table.insert(uri.clone(), CompiledDiagram { svg, width: size.width(), height: size.height() });
                            }
                            Err(e) => eprintln!("warning: failed to parse embedded {} image: {e}", data_uri_label(uri)),
                        },
                        Err(e) => eprintln!("warning: embedded {} image is not valid UTF-8: {e}", data_uri_label(uri)),
                    },
                    Err(e) => eprintln!("warning: refusing to load embedded {} image: {e}", data_uri_label(uri)),
                }
            }
            BlockNode::Blockquote { content } => collect_svgs(content, base_dir, canonical_base, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect_svgs(item, base_dir, canonical_base, table);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect_svgs(column, base_dir, canonical_base, table);
                }
            }
            _ => {}
        }
    }
}
