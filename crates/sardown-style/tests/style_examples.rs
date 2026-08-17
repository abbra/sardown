use sardown_style::{Color, LabelStyle, Stylesheet};

fn example_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/style-examples")).join(name)
}

#[test]
fn us_letter_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("us-letter.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 279.4));
    assert_eq!(sheet.page.margin_mm, 25.4);
    assert_eq!(sheet.typography.font_family, "Times New Roman");
    assert_eq!(sheet.typography.body_size_pt, 12.0);
    assert!(sheet.footer.enabled);
    assert!(!sheet.footer.suppress_on_chapter_start, "US business/academic docs show the page number on every page, including the first");
    assert_eq!(sheet.footer.uniform.center, "{page}");
}

#[test]
fn us_legal_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("us-legal.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 355.6));
    assert_eq!(sheet.page.margin_mm, 25.4);
    assert_eq!(sheet.typography.font_family, "Times New Roman");
    assert_eq!(sheet.footer.uniform.center, "Page {page} of {total_pages}", "legal filings conventionally use \"Page X of Y\"");
}

#[test]
fn eu_a4_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a4.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (210.0, 297.0));
    assert_eq!(sheet.page.margin_mm, 20.0);
    assert_eq!(sheet.typography.font_family, "Helvetica");
    assert_eq!(sheet.typography.body_size_pt, 11.0);
    assert_eq!(sheet.heading.color, Color([0x2c, 0x3e, 0x50]));
    assert!(sheet.footer.enabled);
    assert!(!sheet.footer.suppress_on_chapter_start);
    assert_eq!(sheet.footer.uniform.right, "{page} / {total_pages}", "EU/DIN convention uses a bottom-right \"page / total\" footer");
}

#[test]
fn eu_a3_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a3.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (297.0, 420.0));
    assert_eq!(sheet.page.margin_mm, 20.0);
    assert_eq!(sheet.typography.body_size_pt, 13.0, "A3's larger sheet gets a larger body size than eu-a4");
    assert_eq!(sheet.footer.uniform.right, "{page} / {total_pages}");
}

#[test]
fn eu_a5_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a5.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (148.0, 210.0));
    assert_eq!(sheet.page.margin_mm, 15.0);
    assert_eq!(sheet.typography.body_size_pt, 10.0, "A5's smaller booklet page gets a smaller body size than eu-a4");
    assert_eq!(sheet.footer.uniform.right, "{page} / {total_pages}");
}

#[test]
fn university_paper_preset_uses_apa_style_running_header_and_block_quote_indent() {
    let sheet = Stylesheet::load(&example_path("university-paper.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 279.4));
    assert_eq!(sheet.page.margin_mm, 25.4);
    assert_eq!(sheet.typography.font_family, "Times New Roman");
    assert_eq!(sheet.typography.body_size_pt, 12.0);
    assert_eq!(sheet.blockquote.indent_pt, 36.0, "APA/MLA block quotes use a 0.5in (36pt) indent");
    assert!(sheet.header.enabled);
    assert!(!sheet.header.suppress_on_chapter_start, "APA shows the page number on every page including the first");
    assert_eq!(sheet.header.uniform.right, "{page}");
    assert!(!sheet.footer.enabled, "APA student format uses a header page number, not a footer");
}

#[test]
fn technical_manual_preset_pairs_a_light_syntax_theme_with_a_matching_background() {
    let sheet = Stylesheet::load(&example_path("technical-manual.toml")).unwrap();
    assert_eq!(sheet.page.margin_mm, 20.0);
    assert_eq!(sheet.typography.font_family, "Helvetica");
    assert_eq!(sheet.typography.body_size_pt, 10.5);
    assert_eq!(sheet.heading.color, Color([0x0b, 0x3d, 0x66]));
    assert_eq!(sheet.code_block.syntax_theme, "Solarized (light)");
    assert_eq!(sheet.code_block.label_style, LabelStyle::HeaderBar);
    assert_eq!(sheet.code_block.default.background, Color([0xfd, 0xf6, 0xe3]), "should mirror Solarized Light's own background exactly");
    assert!(sheet.footer.enabled);
    assert_eq!(sheet.footer.uniform.left, "{h1}");
    assert_eq!(sheet.footer.uniform.right, "{page}");
    assert!(sheet.footer.suppress_on_chapter_start, "printed manuals conventionally leave the footer off a chapter's own opening page");
}

#[test]
fn technical_guide_preset_pairs_a_dark_syntax_theme_with_a_matching_dark_background() {
    let sheet = Stylesheet::load(&example_path("technical-guide.toml")).unwrap();
    assert_eq!(sheet.typography.font_family, "Helvetica");
    assert_eq!(sheet.heading.color, Color([0x0e, 0x8a, 0x6d]));
    assert_eq!(sheet.heading.space_before_factor, 1.2);
    assert_eq!(sheet.code_block.syntax_theme, "base16-ocean.dark");
    assert_eq!(sheet.code_block.label_style, LabelStyle::Inline);
    assert_eq!(
        sheet.code_block.default.background,
        Color([0x2b, 0x30, 0x3b]),
        "must match base16-ocean.dark's own background exactly -- this renderer only takes foreground colors from the syntect theme, so a mismatched light background here would make the theme's text unreadable"
    );
    assert_eq!(sheet.code_block.default.label_color, Color([0xc0, 0xc5, 0xce]));
    assert_eq!(sheet.code_block.default.label_background, Color([0x3b, 0x42, 0x52]));
}

#[test]
fn fiction_preset_uses_a_paperback_trim_size_and_suppresses_the_footer_on_chapter_openers() {
    let sheet = Stylesheet::load(&example_path("fiction.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (148.0, 210.0), "A5 approximates a common paperback trim size");
    assert_eq!(sheet.page.margin_mm, 18.0);
    assert_eq!(sheet.typography.font_family, "Garamond");
    assert_eq!(sheet.typography.body_size_pt, 12.5);
    assert_eq!(sheet.heading.space_before_factor, 2.5, "chapters conventionally start partway down the page");
    assert!(!sheet.header.enabled, "novels aren't printed with a running header");
    assert!(sheet.footer.enabled);
    assert_eq!(sheet.footer.uniform.center, "{page}");
    assert!(sheet.footer.suppress_on_chapter_start, "printed novels conventionally leave the page number off a chapter's own opening page");
}
