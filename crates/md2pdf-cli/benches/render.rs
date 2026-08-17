use criterion::{criterion_group, criterion_main, Criterion};
use std::fmt::Write;

fn generate_book(chapters: usize) -> String {
    let mut doc = String::new();
    for i in 0..chapters {
        writeln!(doc, "# Chapter {i}\n").unwrap();
        writeln!(doc, "This is a paragraph of body text for chapter {i}, with **bold** and *italic* words to exercise inline styling. It repeats enough to fill roughly half a page of a typical US Letter document at 12pt body text.\n").unwrap();
        writeln!(doc, "```rust\nfn chapter_{i}() {{\n    println!(\"chapter {i}\");\n}}\n```\n").unwrap();
        writeln!(doc, "| Metric | Value |\n|---|---|\n| Chapter | {i} |\n| Status | done |\n").unwrap();
    }
    doc
}

fn bench_full_render(c: &mut Criterion) {
    let markdown = generate_book(200); // one "chapter" section roughly maps to one page at this content density
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/large-book.md"), &markdown).unwrap();

    c.bench_function("render_200_page_book", |b| {
        b.iter(|| {
            let ast = md2pdf_ast::parse(&markdown);
            let ast = md2pdf_enrich::Highlighter::new().highlight(ast);
            let mut font_db = fontdb::Database::new();
            font_db.load_system_fonts();
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);
            let diagrams = md2pdf_enrich::compile_diagrams(&ast);
            let output = md2pdf_layout::layout(
                &ast,
                &md2pdf_layout::PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4, ..Default::default() },
                &mut font_system,
                std::path::Path::new("."),
                &diagrams,
            );
            md2pdf_pdf::render_pdf(
                &output.pages,
                font_system.db(),
                &output.images,
                &output.diagrams,
                &output.anchors,
                output.page_width_pt,
                output.page_height_pt,
                &output.toc_entries,
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_full_render);
criterion_main!(benches);
