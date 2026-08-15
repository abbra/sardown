use clap::{Parser, Subcommand};
use md2pdf_enrich::Highlighter;
use md2pdf_layout::{layout, PageGeometry};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "md2pdf")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a Markdown file to PDF
    Render {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Render an mdBook source tree (a directory containing book.toml and/or src/SUMMARY.md) to
    /// one combined PDF
    RenderBook {
        book_root: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn us_letter() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4 }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Render { input, output } => {
            let markdown = std::fs::read_to_string(&input)?;
            let ast = md2pdf_ast::parse(&markdown);

            let highlighter = Highlighter::new();
            let ast = highlighter.highlight(ast);
            let diagrams = md2pdf_enrich::compile_diagrams(&ast);

            let base_dir = input.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();

            let mut font_db = fontdb::Database::new();
            font_db.load_system_fonts();
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

            let output_layout = layout(&ast, &us_letter(), &mut font_system, &base_dir, &diagrams);
            let pdf_bytes = md2pdf_pdf::render_pdf(
                &output_layout.pages,
                font_system.db(),
                &output_layout.images,
                &diagrams,
                &output_layout.anchors,
            )?;
            std::fs::write(&output, pdf_bytes)?;
            Ok(())
        }
        Commands::RenderBook { book_root, output } => {
            let ast = md2pdf_book::load_book(&book_root)?;

            let highlighter = Highlighter::new();
            let ast = highlighter.highlight(ast);
            let diagrams = md2pdf_enrich::compile_diagrams(&ast);

            let mut font_db = fontdb::Database::new();
            font_db.load_system_fonts();
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

            // Every embedded image path was already rewritten to absolute during load_book (each
            // chapter can live in a different subdirectory), so the base_dir passed here is never
            // actually joined onto anything -- "." is just a placeholder satisfying the signature.
            let output_layout = layout(&ast, &us_letter(), &mut font_system, std::path::Path::new("."), &diagrams);
            let pdf_bytes = md2pdf_pdf::render_pdf(
                &output_layout.pages,
                font_system.db(),
                &output_layout.images,
                &diagrams,
                &output_layout.anchors,
            )?;
            std::fs::write(&output, pdf_bytes)?;
            Ok(())
        }
    }
}
