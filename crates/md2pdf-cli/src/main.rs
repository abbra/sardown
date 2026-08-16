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

/// Prints `label`, runs `f`, then reports how long it took -- on stderr, so it never mixes with
/// piped/redirected output. A large book's render has no other feedback for several seconds at a
/// time otherwise, which reads as a hang rather than progress.
fn timed_stage<T>(label: &str, f: impl FnOnce() -> T) -> T {
    eprint!("{label}... ");
    let start = std::time::Instant::now();
    let result = f();
    eprintln!("done ({:.2}s)", start.elapsed().as_secs_f64());
    result
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Render { input, output } => {
            let markdown = std::fs::read_to_string(&input)?;
            let mut ast = timed_stage("Parsing markdown", || md2pdf_ast::parse(&markdown));
            md2pdf_ast::tag_diagram_origins(&mut ast, &input);

            let highlighter = Highlighter::new();
            let ast = timed_stage("Highlighting code blocks", || highlighter.highlight(ast));
            let diagrams = timed_stage("Compiling diagrams", || md2pdf_enrich::compile_diagrams(&ast));

            let base_dir = input.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();

            let mut font_db = fontdb::Database::new();
            timed_stage("Loading fonts", || font_db.load_system_fonts());
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

            let output_layout =
                timed_stage("Laying out pages", || layout(&ast, &us_letter(), &mut font_system, &base_dir, &diagrams));
            let pdf_bytes = timed_stage("Rendering PDF", || {
                md2pdf_pdf::render_pdf(&output_layout.pages, font_system.db(), &output_layout.images, &diagrams, &output_layout.anchors)
            })?;
            timed_stage("Writing output", || std::fs::write(&output, pdf_bytes))?;
            eprintln!("Wrote {} ({} pages)", output.display(), output_layout.pages.len());
            Ok(())
        }
        Commands::RenderBook { book_root, output } => {
            let ast = timed_stage("Loading book", || md2pdf_book::load_book(&book_root))?;

            let highlighter = Highlighter::new();
            let ast = timed_stage("Highlighting code blocks", || highlighter.highlight(ast));
            let diagrams = timed_stage("Compiling diagrams", || md2pdf_enrich::compile_diagrams(&ast));

            let mut font_db = fontdb::Database::new();
            timed_stage("Loading fonts", || font_db.load_system_fonts());
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

            // Every embedded image path was already rewritten to absolute during load_book (each
            // chapter can live in a different subdirectory), so base_dir is never actually
            // joined onto anything -- but decode_images also uses it as a security boundary,
            // rejecting any absolute path that isn't one of its descendants. Passing "." there
            // (the CLI process's own CWD) silently dropped every image in any book that didn't
            // happen to live under the current directory; book_root is the real boundary.
            let output_layout =
                timed_stage("Laying out pages", || layout(&ast, &us_letter(), &mut font_system, &book_root, &diagrams));
            let pdf_bytes = timed_stage("Rendering PDF", || {
                md2pdf_pdf::render_pdf(&output_layout.pages, font_system.db(), &output_layout.images, &diagrams, &output_layout.anchors)
            })?;
            timed_stage("Writing output", || std::fs::write(&output, pdf_bytes))?;
            eprintln!("Wrote {} ({} pages)", output.display(), output_layout.pages.len());
            Ok(())
        }
    }
}
