use sardown_ast::{parse_with_slugs, parse_with_style, BlockNode, SlugGenerator};
use sardown_style::{Color, HeadingLevelStyle, Stylesheet};

#[test]
fn heading_uses_the_configured_level_override_size_and_color() {
    let mut style = Stylesheet::default();
    style.heading.levels.insert(
        "1".to_string(),
        HeadingLevelStyle { size_pt: Some(40.0), color: Some(Color([255, 0, 0])), font_family: None, underline_width_pt: None, underline_color: None },
    );
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("# Title\n", &mut slugs, &mut next_id, &style);
    let BlockNode::Heading { content, .. } = &blocks[0] else { panic!("expected a heading, got {:?}", blocks[0]) };
    assert_eq!(content[0].style.size, 40.0);
    assert_eq!(content[0].style.color, [255, 0, 0]);
}

#[test]
fn heading_level_without_an_explicit_override_still_uses_the_built_in_size_table() {
    let style = Stylesheet::default();
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("## Sub\n", &mut slugs, &mut next_id, &style);
    let BlockNode::Heading { content, .. } = &blocks[0] else { panic!("expected a heading, got {:?}", blocks[0]) };
    assert_eq!(content[0].style.size, 22.0); // level 2's built-in default
}

#[test]
fn body_paragraph_uses_the_configured_body_size_and_color() {
    let mut style = Stylesheet::default();
    style.typography.body_size_pt = 14.0;
    style.typography.body_color = Color([10, 20, 30]);
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("Some text.\n", &mut slugs, &mut next_id, &style);
    let BlockNode::Paragraph { content } = &blocks[0] else { panic!("expected a paragraph, got {:?}", blocks[0]) };
    assert_eq!(content[0].style.size, 14.0);
    assert_eq!(content[0].style.color, [10, 20, 30]);
}

#[test]
fn a_tight_list_items_implicit_paragraph_also_uses_the_configured_body_style() {
    let mut style = Stylesheet::default();
    style.typography.body_size_pt = 16.0;
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("- one\n- two\n", &mut slugs, &mut next_id, &style);
    let BlockNode::List { items, .. } = &blocks[0] else { panic!("expected a list, got {:?}", blocks[0]) };
    let BlockNode::Paragraph { content } = &items[0][0] else { panic!("expected an implicit paragraph") };
    assert_eq!(content[0].style.size, 16.0);
}

#[test]
fn a_configured_link_color_applies_to_link_runs_only() {
    let mut style = Stylesheet::default();
    style.typography.body_color = Color([10, 20, 30]);
    style.typography.link_color = Some(Color([0, 104, 158]));
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("Plain and a [link](https://example.com).\n", &mut slugs, &mut next_id, &style);
    let BlockNode::Paragraph { content } = &blocks[0] else { panic!("expected a paragraph, got {:?}", blocks[0]) };
    let link = content.iter().find(|n| n.link_target.is_some()).expect("expected a link run");
    assert_eq!(link.style.color, [0, 104, 158]);
    let plain = content.iter().find(|n| n.link_target.is_none()).expect("expected a plain run");
    assert_eq!(plain.style.color, [10, 20, 30], "non-link runs keep the body color");
}

#[test]
fn links_keep_the_body_color_when_no_link_color_is_configured() {
    let mut style = Stylesheet::default();
    style.typography.body_color = Color([10, 20, 30]);
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let blocks = parse_with_style("A [link](#target) here.\n", &mut slugs, &mut next_id, &style);
    let BlockNode::Paragraph { content } = &blocks[0] else { panic!("expected a paragraph, got {:?}", blocks[0]) };
    let link = content.iter().find(|n| n.link_target.is_some()).expect("expected a link run");
    assert_eq!(link.style.color, [10, 20, 30]);
}

#[test]
fn parse_with_slugs_matches_parse_with_style_using_stylesheet_defaults() {
    let mut slugs_a = SlugGenerator::new();
    let mut next_id_a = 0;
    let via_style = parse_with_style("# T\n\nBody\n\n- item\n", &mut slugs_a, &mut next_id_a, &Stylesheet::default());

    let mut slugs_b = SlugGenerator::new();
    let mut next_id_b = 0;
    let via_slugs = parse_with_slugs("# T\n\nBody\n\n- item\n", &mut slugs_b, &mut next_id_b);

    assert_eq!(via_style, via_slugs);
}

#[test]
fn table_cell_uses_the_configured_table_text_size() {
    let mut style = Stylesheet::default();
    style.table.text_size_pt = 8.0;
    let mut slugs = SlugGenerator::new();
    let mut next_id = 0;
    let md = "| A |\n|---|\n| one |\n";
    let blocks = parse_with_style(md, &mut slugs, &mut next_id, &style);
    let BlockNode::Table { headers, rows, .. } = &blocks[0] else { panic!("expected a table, got {:?}", blocks[0]) };
    assert_eq!(headers[0][0].style.size, 8.0);
    assert_eq!(rows[0][0][0].style.size, 8.0);
}
