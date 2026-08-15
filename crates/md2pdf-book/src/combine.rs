use md2pdf_ast::{BlockNode, ImageSource, InlineNode, SlugGenerator};
use std::path::Path;

use crate::summary::{parse_summary, SummaryItem};

/// Loads an mdBook project rooted at `book_root` into one combined `Vec<BlockNode>`: every
/// chapter listed in `SUMMARY.md`, depth-first in listing order, each starting with a
/// `BlockNode::PageBreak` and (if the chapter file has no top-level heading of its own) a
/// heading synthesized from its `SUMMARY.md` title. Cross-chapter link resolution and the table
/// of contents are later phases -- this only concatenates.
pub fn load_book(book_root: &Path) -> anyhow::Result<Vec<BlockNode>> {
    let src_dir = crate::book_toml::resolve_src_dir(book_root);
    let summary_path = src_dir.join("SUMMARY.md");
    let summary_text = std::fs::read_to_string(&summary_path).map_err(|e| anyhow::anyhow!("failed to read {}: {e}", summary_path.display()))?;
    let summary = parse_summary(&summary_text);

    let mut slugs = SlugGenerator::new();
    let mut next_diagram_id = 0usize;
    let mut combined = Vec::new();
    collect_chapters(&summary.items, &src_dir, &mut slugs, &mut next_diagram_id, &mut combined);
    Ok(combined)
}

fn collect_chapters(
    items: &[SummaryItem],
    src_dir: &Path,
    slugs: &mut SlugGenerator,
    next_diagram_id: &mut usize,
    out: &mut Vec<BlockNode>,
) {
    for item in items {
        match item {
            SummaryItem::Chapter { title, path, children } => {
                if let Some(rel_path) = path {
                    let chapter_path = src_dir.join(rel_path);
                    match std::fs::read_to_string(&chapter_path) {
                        Ok(text) => {
                            let mut blocks = md2pdf_ast::parse_with_slugs(&text, slugs, next_diagram_id);
                            let chapter_dir = chapter_path.parent().unwrap_or(src_dir).to_path_buf();
                            absolutize_image_paths(&mut blocks, &chapter_dir);
                            prepend_chapter_start(&mut blocks, title, slugs);
                            out.extend(blocks);
                        }
                        Err(e) => {
                            eprintln!("warning: failed to read chapter {}: {e}", chapter_path.display());
                        }
                    }
                }
                // Recurse into children regardless of whether this entry itself had content --
                // mdBook allows a draft parent (no link) with real, linked sub-chapters.
                collect_chapters(children, src_dir, slugs, next_diagram_id, out);
            }
            SummaryItem::PartTitle(_) | SummaryItem::Separator => {
                // No visual treatment in the combined body in this phase -- these matter for
                // the table of contents (a later phase), not chapter concatenation.
            }
        }
    }
}

/// Rewrites this chapter's own embedded image paths to absolute. Each chapter can live in a
/// different subdirectory of the book, so a single global base_dir (as single-file rendering
/// uses) can't resolve every chapter's images correctly; an already-absolute path makes the
/// existing `base_dir.join(path)` in md2pdf-layout's `decode_images` a no-op later, so nothing
/// downstream needs to change.
fn absolutize_image_paths(blocks: &mut [BlockNode], chapter_dir: &Path) {
    for block in blocks {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } => {
                if !path.is_absolute() {
                    *path = chapter_dir.join(&path);
                }
            }
            BlockNode::Blockquote { content } => absolutize_image_paths(content, chapter_dir),
            BlockNode::List { items, .. } => {
                for item in items {
                    absolutize_image_paths(item, chapter_dir);
                }
            }
            _ => {}
        }
    }
}

/// Prepends a page break, and (if the chapter has no top-level heading of its own) a heading
/// synthesized from its SUMMARY.md title so every chapter has *something* to identify it at
/// the top of its page even if the source file jumps straight into body text.
fn prepend_chapter_start(blocks: &mut Vec<BlockNode>, title: &str, slugs: &mut SlugGenerator) {
    let needs_heading = !matches!(blocks.first(), Some(BlockNode::Heading { .. }));
    let mut prefix = vec![BlockNode::PageBreak];
    if needs_heading {
        let id = slugs.generate(title);
        prefix.push(BlockNode::Heading {
            level: 1,
            id,
            content: vec![InlineNode {
                text: title.to_string(),
                style: md2pdf_ast::heading_style_for_level(1),
                link_target: None,
            }],
        });
    }
    blocks.splice(0..0, prefix);
}
