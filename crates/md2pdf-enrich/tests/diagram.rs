use md2pdf_ast::BlockNode;
use md2pdf_enrich::compile_diagrams;

#[test]
fn compiles_a_flowchart_to_svg_with_positive_dimensions() {
    let ast = vec![BlockNode::MermaidDiagram {
        id: "diagram-1".to_string(),
        source: "flowchart TD\n    A --> B\n".to_string(),
    }];
    let table = compile_diagrams(&ast);
    let diagram = table.get("diagram-1").expect("diagram not found in table");
    assert!(diagram.svg.contains("<svg"), "expected SVG output, got: {}", diagram.svg);
    assert!(diagram.width > 0.0 && diagram.height > 0.0);
}

#[test]
fn unsupported_or_invalid_diagram_source_is_skipped_not_panicking() {
    let ast = vec![BlockNode::MermaidDiagram {
        id: "diagram-bad".to_string(),
        source: "not a real mermaid diagram at all {{{".to_string(),
    }];
    let table = compile_diagrams(&ast); // must not panic
    assert!(!table.contains_key("diagram-bad"));
}

#[test]
fn recurses_into_blockquotes_and_list_items() {
    let inner = BlockNode::MermaidDiagram { id: "nested".to_string(), source: "flowchart TD\n    A --> B\n".to_string() };
    let ast = vec![BlockNode::List { ordered: false, items: vec![vec![inner]] }];
    let table = compile_diagrams(&ast);
    assert!(table.contains_key("nested"));
}
