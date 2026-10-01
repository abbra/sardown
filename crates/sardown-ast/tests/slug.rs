use sardown_ast::SlugGenerator;

#[test]
fn slugifies_basic_text() {
    let mut slug_gen = SlugGenerator::new();
    assert_eq!(slug_gen.generate("Hello, World!"), "hello-world");
}

#[test]
fn collapses_repeated_separators_and_trims() {
    let mut slug_gen = SlugGenerator::new();
    assert_eq!(slug_gen.generate("  --Foo   Bar--  "), "foo-bar");
}

#[test]
fn appends_numeric_suffix_on_collision() {
    let mut slug_gen = SlugGenerator::new();
    assert_eq!(slug_gen.generate("Overview"), "overview");
    assert_eq!(slug_gen.generate("Overview"), "overview-1");
    assert_eq!(slug_gen.generate("Overview"), "overview-2");
}
