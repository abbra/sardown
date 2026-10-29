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

#[cfg(feature = "ondine")]
#[test]
fn the_ondine_renderer_draws_a_diagram_its_own_way() {
    let mut style = sardown_style::Stylesheet::default();
    style.mermaid.renderer = sardown_style::MermaidRenderer::Ondine;
    let ast =
        vec![BlockNode::MermaidDiagram { id: "diagram-1".to_string(), source: "sequenceDiagram\n    A->>B: hi\n".to_string(), line: 1, column: 1, file: None }];
    let table = sardown_enrich::compile_diagrams_styled(&ast, &test_svg_options(), &style);
    let diagram = table.get("diagram-1").expect("ondine should have drawn the diagram");
    assert!(diagram.width > 0.0 && diagram.height > 0.0);
}

/// A pie, mindmap and chart color their data through Ondine's `--ondine-series-*` variables,
/// whose defaults are `color-mix()` expressions that neither `css_vars` nor `usvg` evaluates --
/// unoverridden, every slice draws in the same color.
#[cfg(feature = "ondine")]
#[test]
fn the_ondine_renderer_gives_each_chart_series_its_own_color() {
    let ast = vec![BlockNode::MermaidDiagram {
        id: "pie".to_string(),
        source: "pie title Pets\n    \"Dogs\" : 38\n    \"Cats\" : 27\n    \"Foxes\" : 35\n".to_string(),
        line: 1,
        column: 1,
        file: None,
    }];
    let table = sardown_enrich::compile_diagrams_styled(&ast, &test_svg_options(), &sardown_style::Stylesheet::default());
    let mut colors = std::collections::HashSet::new();
    table["pie"].tree.root().children().iter().for_each(|node| collect_fill_colors(node, &mut colors));
    assert!(colors.len() >= 3, "expected a distinct fill per pie series, got {colors:?}");
}

#[cfg(feature = "ondine")]
fn collect_fill_colors(node: &usvg::Node, colors: &mut std::collections::HashSet<(u8, u8, u8)>) {
    if let usvg::Node::Path(path) = node {
        if let Some(usvg::Paint::Color(color)) = path.fill().map(|fill| fill.paint()) {
            colors.insert((color.red, color.green, color.blue));
        }
    }
    if let usvg::Node::Group(group) = node {
        for child in group.children() {
            collect_fill_colors(child, colors);
        }
    }
}
