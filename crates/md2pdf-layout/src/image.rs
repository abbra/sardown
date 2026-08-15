use md2pdf_ast::{BlockNode, ImageSource};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolves `path` against `base_dir` and rejects the result unless it stays within `base_dir`.
///
/// `path` comes directly from Markdown authored by whoever wrote the document being rendered —
/// without this check, `![x](../../../etc/passwd)` (relative traversal) or `![x](/etc/passwd)`
/// (an absolute path, which `Path::join` uses verbatim, discarding `base_dir` entirely) would let
/// a document embed arbitrary local files into the output PDF. Canonicalizing resolves `..` and
/// symlinks via the OS, so the `starts_with` check can't be fooled by a symlink that lexically
/// looks contained but points outside `base_dir`.
fn resolve_within_base(base_dir: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical_base = base_dir.canonicalize().map_err(|e| format!("cannot resolve base directory {}: {e}", base_dir.display()))?;
    let candidate = base_dir.join(path);
    let canonical_candidate = candidate.canonicalize().map_err(|e| format!("cannot resolve path: {e}"))?;
    if canonical_candidate.starts_with(&canonical_base) {
        Ok(canonical_candidate)
    } else {
        Err(format!("path escapes base directory {}", canonical_base.display()))
    }
}

pub struct DecodedImage {
    pub rgba8: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub type ImageTable = HashMap<String, DecodedImage>;

pub fn decode_images(ast: &[BlockNode], base_dir: &Path) -> ImageTable {
    let mut table = HashMap::new();
    collect(ast, base_dir, &mut table);
    table
}

fn collect(ast: &[BlockNode], base_dir: &Path, table: &mut ImageTable) {
    for block in ast {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } => {
                let key = path.to_string_lossy().to_string();
                if table.contains_key(&key) {
                    continue;
                }
                match resolve_within_base(base_dir, path) {
                    Ok(resolved) => match image::open(&resolved) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            table.insert(key, DecodedImage { width: rgba.width(), height: rgba.height(), rgba8: rgba.into_raw() });
                        }
                        Err(e) => eprintln!("warning: failed to decode image {key}: {e}"),
                    },
                    Err(e) => eprintln!("warning: refusing to load image {key}: {e}"),
                }
            }
            BlockNode::Image { source: ImageSource::External(url), .. } => {
                eprintln!("warning: skipping external image (not fetched): {url}");
            }
            BlockNode::Blockquote { content } => collect(content, base_dir, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, base_dir, table);
                }
            }
            _ => {}
        }
    }
}
