use sardown_ast::BlockNode;

const DIRECTIVE_PREFIX: &str = "@layout:";

/// One slide's own content, plus the name of the layout its `@layout:` directive requested (if
/// any). `layout_name: None` means "use `[slides].default_layout`", resolved later by
/// `resolve_layout`.
#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub layout_name: Option<String>,
    pub blocks: Vec<BlockNode>,
}

/// Splits a whole deck's parsed blocks into one `Slide` per top-level `---`
/// (`BlockNode::ThematicBreak`), then extracts each slide's own leading `@layout: <name>`
/// directive paragraph (if present). A deck with zero thematic breaks is exactly one slide.
pub fn split_into_slides(ast: Vec<BlockNode>) -> Vec<Slide> {
    let mut slides = Vec::new();
    let mut current = Vec::new();
    for block in ast {
        match block {
            BlockNode::ThematicBreak => slides.push(finish_slide(std::mem::take(&mut current))),
            other => current.push(other),
        }
    }
    slides.push(finish_slide(current));
    slides
}

fn finish_slide(mut blocks: Vec<BlockNode>) -> Slide {
    let layout_name = extract_layout_directive(&mut blocks);
    Slide { layout_name, blocks }
}

/// If `blocks`'s first element is a `Paragraph` whose concatenated inline text (trimmed) is
/// exactly `@layout: <name>`, removes that paragraph and returns the trimmed `<name>`.
fn extract_layout_directive(blocks: &mut Vec<BlockNode>) -> Option<String> {
    let Some(BlockNode::Paragraph { content }) = blocks.first() else { return None };
    let text: String = content.iter().map(|n| n.text.as_str()).collect();
    let name = text.trim().strip_prefix(DIRECTIVE_PREFIX)?.trim();
    if name.is_empty() {
        return None;
    }
    let name = name.to_string();
    blocks.remove(0);
    Some(name)
}
