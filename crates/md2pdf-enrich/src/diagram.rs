use md2pdf_ast::BlockNode;
use merman::svg::HeadlessRenderer;
use std::collections::HashMap;

#[derive(Clone)]
pub struct CompiledDiagram {
    pub svg: String,
    pub width: f32,
    pub height: f32,
}

pub type DiagramTable = HashMap<String, CompiledDiagram>;

pub fn compile_diagrams(ast: &[BlockNode]) -> DiagramTable {
    let renderer = HeadlessRenderer::new();
    let mut table = HashMap::new();
    collect(ast, &renderer, &mut table);
    table
}

fn collect(ast: &[BlockNode], renderer: &HeadlessRenderer, table: &mut DiagramTable) {
    for block in ast {
        match block {
            BlockNode::MermaidDiagram { id, source, line, column, file } => {
                let fence_location = |line: usize, column: usize| match file {
                    Some(f) => format!("{}:{line}:{column}", f.display()),
                    None => format!("line {line}, column {column}"),
                };
                match renderer.render_resvg_compatible_svg_sync(source) {
                    Ok(Some(resvg_safe_svg)) => {
                        let svg = resvg_safe_svg.into_string();
                        match usvg::Tree::from_str(&svg, &usvg::Options::default()) {
                            Ok(tree) => {
                                let size = tree.size();
                                table.insert(id.clone(), CompiledDiagram { svg, width: size.width(), height: size.height() });
                            }
                            Err(e) => {
                                eprintln!("warning: merman produced unparseable SVG for the Mermaid diagram at {}: {e}", fence_location(*line, *column))
                            }
                        }
                    }
                    Ok(None) => eprintln!("warning: merman produced no output for the Mermaid diagram at {}", fence_location(*line, *column)),
                    Err(e) => {
                        // merman's DiagramParse errors carry a byte span *inside the diagram's
                        // own source* pointing at the actual offending token -- reporting only
                        // the opening fence's location (the fallback below) left no way to find
                        // the real problem in anything but a one-line diagram.
                        let location = match diagram_parse_span(&e) {
                            Some(span) => {
                                let (inner_line, inner_column) = line_col_at(source, span.start);
                                fence_location(line + inner_line, inner_column)
                            }
                            None => fence_location(*line, *column),
                        };
                        eprintln!("warning: failed to render the Mermaid diagram at {location}: {e}");
                    }
                }
            }
            BlockNode::Blockquote { content } => collect(content, renderer, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, renderer, table);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect(column, renderer, table);
                }
            }
            _ => {}
        }
    }
}

fn diagram_parse_span(error: &merman::svg::HeadlessError) -> Option<merman::SourceSpan> {
    match error {
        merman::svg::HeadlessError::Parse(merman::Error::DiagramParse { diagnostic, .. }) => diagnostic.span(),
        _ => None,
    }
}

/// 1-indexed (line, column) for a byte offset into `text`. Column counts characters (not bytes)
/// since the start of the line. Mirrors md2pdf-ast's identical helper for the same purpose --
/// small and self-contained enough that a shared crate for just this isn't worth it.
fn line_col_at(text: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &text[..byte_offset.min(text.len())];
    let line = prefix.matches('\n').count() + 1;
    let column = match prefix.rfind('\n') {
        Some(i) => prefix[i + 1..].chars().count() + 1,
        None => prefix.chars().count() + 1,
    };
    (line, column)
}
