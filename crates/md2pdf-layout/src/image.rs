use md2pdf_ast::{BlockNode, ImageSource};
use md2pdf_enrich::{CompiledDiagram, DiagramTable};
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

fn is_svg_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

fn collect(ast: &[BlockNode], base_dir: &Path, table: &mut ImageTable) {
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
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect(column, base_dir, table);
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
    collect_svgs(ast, base_dir, &mut table);
    table
}

fn collect_svgs(ast: &[BlockNode], base_dir: &Path, table: &mut DiagramTable) {
    for block in ast {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } if is_svg_path(path) => {
                let key = path.to_string_lossy().to_string();
                if table.contains_key(&key) {
                    continue;
                }
                match resolve_within_base(base_dir, path) {
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
            BlockNode::Blockquote { content } => collect_svgs(content, base_dir, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect_svgs(item, base_dir, table);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect_svgs(column, base_dir, table);
                }
            }
            _ => {}
        }
    }
}
