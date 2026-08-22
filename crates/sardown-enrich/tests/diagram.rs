use sardown_ast::BlockNode;
use sardown_enrich::compile_diagrams;

/// Diagram compilation only needs *a* fontdb to build render-ready trees; the default empty one
/// is enough for these tests (real renders pass the document's own via svg_tree_options(db)).
fn test_svg_options() -> usvg::Options<'static> {
    usvg::Options::default()
}
#[test]
fn compiles_a_flowchart_to_svg_with_positive_dimensions() {
    let ast =
        vec![BlockNode::MermaidDiagram { id: "diagram-1".to_string(), source: "flowchart TD\n    A --> B\n".to_string(), line: 1, column: 1, file: None }];
    let table = compile_diagrams(&ast, &test_svg_options());
    let diagram = table.get("diagram-1").expect("diagram not found in table");
    assert!(!diagram.tree.root().children().is_empty(), "expected a parsed tree with rendered content");
    assert!(diagram.width > 0.0 && diagram.height > 0.0);
}

#[test]
fn unsupported_or_invalid_diagram_source_is_skipped_not_panicking() {
    let ast = vec![BlockNode::MermaidDiagram {
        id: "diagram-bad".to_string(),
        source: "not a real mermaid diagram at all {{{".to_string(),
        line: 1,
        column: 1,
        file: Some(std::path::PathBuf::from("chapter.md")),
    }];
    let table = compile_diagrams(&ast, &test_svg_options()); // must not panic
    assert!(!table.contains_key("diagram-bad"));
}

#[test]
fn recurses_into_blockquotes_and_list_items() {
    let inner = BlockNode::MermaidDiagram { id: "nested".to_string(), source: "flowchart TD\n    A --> B\n".to_string(), line: 1, column: 1, file: None };
    let ast = vec![BlockNode::List { ordered: false, start: None, items: vec![vec![inner]] }];
    let table = compile_diagrams(&ast, &test_svg_options());
    assert!(table.contains_key("nested"));
}

#[test]
fn recurses_into_columns() {
    let inner = BlockNode::MermaidDiagram { id: "in-column".to_string(), source: "flowchart TD\n    A --> B\n".to_string(), line: 1, column: 1, file: None };
    let ast = vec![BlockNode::Columns(vec![vec![inner]])];
    let table = compile_diagrams(&ast, &test_svg_options());
    assert!(table.contains_key("in-column"));
}
