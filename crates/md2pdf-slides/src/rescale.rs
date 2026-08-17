use md2pdf_ast::{BlockNode, InlineNode};
use md2pdf_style::{SlideLayoutStyle, Stylesheet};

/// Rewrites every `InlineNode.style.size`/`.color` in `blocks` in place, to actually make
/// auto-shrink scaling and a layout's `text_color` override take visual effect.
///
/// `md2pdf_layout::layout_impl` renders a paragraph/heading/table-cell's text at whatever size
/// and color its own `InlineNode.style` already carries -- baked in once, when the whole deck was
/// originally parsed via `md2pdf_ast::parse_with_style`, and never re-read from the `Stylesheet`
/// passed to `layout_impl` at render time (confirmed by reading `render_block`'s `Heading`/
/// `Paragraph`/`Table` arms: each pulls its size straight from `content[0].style.size`/
/// `cell[0].style.size`, not from `cursor.style`). `build_slide_stylesheet`'s sibling module
/// documents which fields *do* get re-read fresh at render time (list markers, table spacing
/// math, code block font sizes) -- everything else needs this direct AST rewrite instead.
///
/// Each block category gets its own target size, matching the same source `base` values that
/// originally produced it, times `scale`: a `Heading`'s target is `base.heading.resolve(level)`
/// (so a base document's own `[heading.levels.N]` overrides still apply, and different heading
/// levels keep their own distinct sizes), a `Paragraph`'s or list item's target is
/// `layout.body_size_pt` if the layout sets one, else `base.typography.body_size_pt`, and a table
/// cell's target is always `base.table.text_size_pt` (`SlideLayoutStyle` has no table-text
/// override). `layout.text_color`, if set, overwrites every category's color uniformly.
pub fn rescale_slide_content(blocks: &mut [BlockNode], base: &Stylesheet, layout: &SlideLayoutStyle, scale: f32) {
    let body_size_pt = layout.body_size_pt.unwrap_or(base.typography.body_size_pt) * scale;
    let table_cell_size_pt = base.table.text_size_pt * scale;
    let text_color = layout.text_color.map(|c| c.0);
    rescale_blocks(blocks, base, scale, body_size_pt, table_cell_size_pt, text_color);
}

fn rescale_blocks(
    blocks: &mut [BlockNode],
    base: &Stylesheet,
    scale: f32,
    body_size_pt: f32,
    table_cell_size_pt: f32,
    text_color: Option<[u8; 3]>,
) {
    for block in blocks {
        match block {
            BlockNode::Heading { level, content, .. } => {
                let size = base.heading.resolve(*level).size_pt * scale;
                set_inline_style(content, size, text_color);
            }
            BlockNode::Paragraph { content } => set_inline_style(content, body_size_pt, text_color),
            BlockNode::Blockquote { content } => {
                rescale_blocks(content, base, scale, body_size_pt, table_cell_size_pt, text_color)
            }
            BlockNode::List { items, .. } => {
                for item in items {
                    rescale_blocks(item, base, scale, body_size_pt, table_cell_size_pt, text_color);
                }
            }
            BlockNode::Table { headers, rows, .. } => {
                for cell in headers.iter_mut() {
                    set_inline_style(cell, table_cell_size_pt, text_color);
                }
                for row in rows.iter_mut() {
                    for cell in row.iter_mut() {
                        set_inline_style(cell, table_cell_size_pt, text_color);
                    }
                }
            }
            BlockNode::CodeBlock { .. } | BlockNode::ThematicBreak | BlockNode::PageBreak | BlockNode::MermaidDiagram { .. } | BlockNode::Image { .. } => {}
        }
    }
}

fn set_inline_style(nodes: &mut [InlineNode], size: f32, text_color: Option<[u8; 3]>) {
    for node in nodes {
        node.style.size = size;
        if let Some(color) = text_color {
            node.style.color = color;
        }
    }
}
