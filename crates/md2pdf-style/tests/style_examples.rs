use md2pdf_style::Stylesheet;

fn example_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/style-examples")).join(name)
}

#[test]
fn us_letter_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("us-letter.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 279.4));
    assert_eq!(sheet.page.margin_mm, 25.4);
}

#[test]
fn us_legal_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("us-legal.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (215.9, 355.6));
    assert_eq!(sheet.page.margin_mm, 25.4);
}

#[test]
fn eu_a4_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a4.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (210.0, 297.0));
    assert_eq!(sheet.page.margin_mm, 20.0);
}

#[test]
fn eu_a3_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a3.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (297.0, 420.0));
    assert_eq!(sheet.page.margin_mm, 20.0);
}

#[test]
fn eu_a5_example_parses_to_the_expected_dimensions_and_margin() {
    let sheet = Stylesheet::load(&example_path("eu-a5.toml")).unwrap();
    assert_eq!(sheet.page.dimensions_mm(), (148.0, 210.0));
    assert_eq!(sheet.page.margin_mm, 15.0);
}
