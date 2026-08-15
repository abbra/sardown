use proptest::prelude::*;

fn arb_inline_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{0,40}"
}

fn arb_node_id() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9]{0,5}"
}

fn arb_mermaid_source() -> impl Strategy<Value = String> {
    proptest::collection::vec((arb_node_id(), arb_node_id()), 1..5).prop_map(|edges| {
        let body: String = edges.into_iter().map(|(a, b)| format!("    {a} --> {b}\n")).collect();
        format!("flowchart TD\n{body}")
    })
}

fn arb_block() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_inline_text().prop_map(|t| format!("# {t}\n\n")),
        arb_inline_text().prop_map(|t| format!("{t}\n\n")),
        arb_inline_text().prop_map(|t| format!("> {t}\n\n")),
        Just("---\n\n".to_string()),
        arb_inline_text().prop_map(|t| format!("```\n{t}\n```\n\n")),
        arb_mermaid_source().prop_map(|s| format!("```mermaid\n{s}```\n\n")),
        (1u8..=3).prop_flat_map(|n| {
            proptest::collection::vec(arb_inline_text(), n as usize).prop_map(|items| items.into_iter().map(|i| format!("- {i}\n")).collect::<String>() + "\n")
        }),
        arb_inline_text().prop_map(|t| format!("| A | B |\n|---|---|\n| {t} | x |\n\n")),
    ]
}

fn arb_document() -> impl Strategy<Value = String> {
    proptest::collection::vec(arb_block(), 0..30).prop_map(|blocks| blocks.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn full_pipeline_never_panics_on_randomized_documents(markdown in arb_document()) {
        let ast = md2pdf_ast::parse(&markdown);
        let ast = md2pdf_enrich::Highlighter::new().highlight(ast);

        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();
        let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

        let diagrams = md2pdf_enrich::compile_diagrams(&ast);
        let output = md2pdf_layout::layout(&ast, &md2pdf_layout::PageGeometry {
            page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4,
        }, &mut font_system, std::path::Path::new("."), &diagrams);

        let pdf_bytes = md2pdf_pdf::render_pdf(&output.pages, font_system.db(), &output.images, &diagrams, &output.anchors);
        prop_assert!(pdf_bytes.is_ok(), "render_pdf returned an error instead of panicking, which is fine, but got: {:?}", pdf_bytes.err());
    }
}
