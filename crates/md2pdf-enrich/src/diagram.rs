use md2pdf_ast::BlockNode;
use merman::svg::HeadlessRenderer;
use std::collections::HashMap;

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
                let location = match file {
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
                            Err(e) => eprintln!("warning: merman produced unparseable SVG for the Mermaid diagram at {location}: {e}"),
                        }
                    }
                    Ok(None) => eprintln!("warning: merman produced no output for the Mermaid diagram at {location}"),
                    Err(e) => eprintln!("warning: failed to render the Mermaid diagram at {location}: {e}"),
                }
            }
            BlockNode::Blockquote { content } => collect(content, renderer, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, renderer, table);
                }
            }
            _ => {}
        }
    }
}
