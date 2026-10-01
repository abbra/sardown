use sardown_ast::{BlockNode, ImageSource, InlineNode, SlugGenerator};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::summary::{SummaryItem, parse_summary};

/// Loads an mdBook project rooted at `book_root` into one combined `Vec<BlockNode>`: every
/// chapter listed in `SUMMARY.md`, depth-first in listing order, each starting with a
/// `BlockNode::PageBreak` and (if the chapter file has no top-level heading of its own) a
/// heading synthesized from its `SUMMARY.md` title, with relative links between chapters
/// resolved into working internal anchors and `{{#include ...}}` directives resolved before
/// parsing.
pub fn load_book(book_root: &Path, style: &sardown_style::Stylesheet) -> anyhow::Result<Vec<BlockNode>> {
    let src_dir = crate::book_toml::resolve_src_dir(book_root);
    let summary_path = src_dir.join("SUMMARY.md");
    let summary_text = std::fs::read_to_string(&summary_path).map_err(|e| anyhow::anyhow!("failed to read {}: {e}", summary_path.display()))?;
    let summary = parse_summary(&summary_text);

    let known_files = crate::crossref::known_chapter_files(&summary.items, &src_dir);

    let mut builder = BookBuilder {
        book_root,
        src_dir: &src_dir,
        known_files: &known_files,
        style,
        slugs: SlugGenerator::new(),
        next_diagram_id: 0,
        slug_map: HashMap::new(),
        chapter_start_map: HashMap::new(),
        out: Vec::new(),
    };
    builder.collect(&summary.items);
    crate::crossref::resolve_links(&mut builder.out, &builder.slug_map, &builder.chapter_start_map);
    Ok(builder.out)
}

/// The shared state of one book assembly: the immutable inputs plus the mutable maps and
/// generators threaded unchanged through every level of the recursive chapter walk. Grouped
/// into one struct (the same pattern sardown-slides' `DeckContext` uses) so the recursion's
/// shared state is explicit and a new cross-chapter concern is a new field, not a new
/// parameter at every call site.
struct BookBuilder<'a> {
    book_root: &'a Path,
    src_dir: &'a Path,
    known_files: &'a HashSet<PathBuf>,
    style: &'a sardown_style::Stylesheet,
    slugs: SlugGenerator,
    next_diagram_id: usize,
    slug_map: HashMap<(Rc<Path>, String), String>,
    chapter_start_map: HashMap<PathBuf, String>,
    out: Vec<BlockNode>,
}

impl BookBuilder<'_> {
    fn collect(&mut self, items: &[SummaryItem]) {
        for item in items {
            match item {
                SummaryItem::Chapter { title, path, children } => {
                    if let Some(rel_path) = path {
                        let chapter_path = self.src_dir.join(rel_path);
                        match std::fs::read_to_string(&chapter_path) {
                            Ok(text) => {
                                let chapter_dir = chapter_path.parent().unwrap_or(self.src_dir).to_path_buf();
                                let text = crate::include::resolve_includes(&text, &chapter_dir, self.book_root);
                                let mut blocks = sardown_ast::parse_with_style(&text, &mut self.slugs, &mut self.next_diagram_id, self.style);
                                // Tagged with the same relative path SUMMARY.md itself names this
                                // chapter by, not the full absolute filesystem path -- that's what
                                // the book's author will actually recognize in a diagram warning.
                                sardown_ast::tag_diagram_origins(&mut blocks, rel_path);
                                absolutize_image_paths(&mut blocks, &chapter_dir);
                                crate::crossref::classify_links(&mut blocks, &chapter_dir, self.known_files);
                                prepend_chapter_start(&mut blocks, title, &mut self.slugs, self.style);

                                // `Rc` so recording every heading's slug shares one path instead of
                                // copying it per heading.
                                let canonical_chapter_path: Rc<Path> = Rc::from(std::fs::canonicalize(&chapter_path).unwrap_or_else(|_| chapter_path.clone()));
                                crate::crossref::record_heading_slugs(&blocks, &canonical_chapter_path, &mut self.slug_map);
                                if let Some(first_heading_id) = first_heading_id(&blocks) {
                                    self.chapter_start_map.insert(canonical_chapter_path.as_ref().to_path_buf(), first_heading_id);
                                }

                                self.out.extend(blocks);
                            }
                            Err(e) => {
                                eprintln!("warning: failed to read chapter {}: {e}", chapter_path.display());
                            }
                        }
                    }
                    // Recurse into children regardless of whether this entry itself had content --
                    // mdBook allows a draft parent (no link) with real, linked sub-chapters.
                    self.collect(children);
                }
                SummaryItem::PartTitle(title) => {
                    self.out.push(BlockNode::PageBreak);
                    self.out.push(synthesized_heading(title, &mut self.slugs, self.style));
                }
                SummaryItem::Separator => {
                    // SUMMARY.md's thematic breaks are a sidebar-only grouping cue with no title
                    // text of their own -- nothing to render as a heading.
                }
            }
        }
    }
}
fn first_heading_id(blocks: &[BlockNode]) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        BlockNode::Heading { id, .. } => Some(id.clone()),
        _ => None,
    })
}

/// Rewrites this chapter's own embedded image paths to absolute. Each chapter can live in a
/// different subdirectory of the book, so a single global base_dir (as single-file rendering
/// uses) can't resolve every chapter's images correctly; an already-absolute path makes the
/// existing `base_dir.join(path)` in sardown-layout's `decode_images` a no-op later, so nothing
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
            BlockNode::Columns(columns) => {
                for column in columns {
                    absolutize_image_paths(column, chapter_dir);
                }
            }
            _ => {}
        }
    }
}

/// Prepends a page break, and (if the chapter has no top-level heading of its own) a heading
/// synthesized from its SUMMARY.md title so every chapter has *something* to identify it at
/// the top of its page even if the source file jumps straight into body text.
fn prepend_chapter_start(blocks: &mut Vec<BlockNode>, title: &str, slugs: &mut SlugGenerator, style: &sardown_style::Stylesheet) {
    let needs_heading = !matches!(blocks.first(), Some(BlockNode::Heading { .. }));
    let mut prefix = vec![BlockNode::PageBreak];
    if needs_heading {
        prefix.push(synthesized_heading(title, slugs, style));
    }
    blocks.splice(0..0, prefix);
}

/// Builds a level-1 `BlockNode::Heading` from plain title text, styled per the stylesheet's own
/// H1 resolution -- shared by chapter-start synthesis and part-title rendering, both of which
/// need a heading that didn't come from parsing an actual source file.
fn synthesized_heading(title: &str, slugs: &mut SlugGenerator, style: &sardown_style::Stylesheet) -> BlockNode {
    let id = slugs.generate(title);
    let resolved = style.heading.resolve(1);
    BlockNode::Heading {
        level: 1,
        id,
        content: vec![InlineNode {
            text: title.to_string(),
            style: sardown_ast::TextStyle {
                bold: false,
                italic: false,
                strikethrough: false,
                size: resolved.size_pt,
                color: resolved.color.0,
                font_family: resolved.font_family.as_str().into(),
            },
            link_target: None,
        }],
    }
}
