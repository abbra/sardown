use crate::BlockNode;

const OPEN: &str = "::columns";
const COLUMN: &str = "::column";
const CLOSE: &str = "::end";

/// Groups `::columns`/`::column`/`::end` sentinel paragraphs into `BlockNode::Columns` nodes.
///
/// Each sentinel is an ordinary one-line paragraph as far as the core parser is concerned (no
/// parser changes needed) -- recognized here by exact, trimmed text match, the same technique
/// `sardown-slides`' `@layout:` directive already uses. `::columns` opens a block; the first
/// `::column` after it starts collecting into column 0; each subsequent `::column` starts a new
/// column; `::end` closes the block. This is a flat state machine with no nesting concept: a
/// `::column`/`::end` seen while *not* inside a columns block, or a `::columns` seen while
/// *already* inside one, is never specially recognized -- it passes through as an ordinary
/// paragraph, exactly like any markdown text that happens to say the same words.
///
/// Content is never dropped, regardless of how a deck author's markup happens to be malformed:
/// a `::columns` block with no `::end` before `blocks` runs out closes there; one with no
/// `::column` markers at all becomes a single implicit column holding everything seen.
pub fn group_columns(blocks: Vec<BlockNode>) -> Vec<BlockNode> {
    let mut result = Vec::new();
    let mut iter = blocks.into_iter();
    while let Some(block) = iter.next() {
        if is_sentinel(&block, OPEN) {
            result.push(BlockNode::Columns(collect_columns(&mut iter)));
        } else {
            result.push(block);
        }
    }
    result
}

fn is_sentinel(block: &BlockNode, text: &str) -> bool {
    match block {
        BlockNode::Paragraph { content } => content.iter().map(|n| n.text.as_str()).collect::<String>().trim() == text,
        _ => false,
    }
}

fn collect_columns(iter: &mut std::vec::IntoIter<BlockNode>) -> Vec<Vec<BlockNode>> {
    let mut columns = Vec::new();
    let mut current = Vec::new();
    let mut started_a_column = false;
    for block in iter.by_ref() {
        if is_sentinel(&block, CLOSE) {
            break;
        }
        if is_sentinel(&block, COLUMN) {
            if started_a_column {
                columns.push(std::mem::take(&mut current));
            }
            started_a_column = true;
            continue;
        }
        current.push(block);
    }
    if started_a_column || !current.is_empty() {
        columns.push(current);
    }
    columns
}
