use clap::{Parser, Subcommand};
use md2pdf_enrich::Highlighter;
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
        /// Path to a stylesheet TOML file. Falls back to built-in defaults if omitted.
        #[arg(long)]
        style: Option<PathBuf>,
        /// Document title, available to header/footer templates as {title}. Overrides
        /// [document].title from the stylesheet if both are given.
        #[arg(long)]
        title: Option<String>,
        /// Document author, available to header/footer templates as {author}. Overrides
        /// [document].author from the stylesheet if both are given.
        #[arg(long)]
        author: Option<String>,
    },
    /// Render an mdBook source tree (a directory containing book.toml and/or src/SUMMARY.md) to
    /// one combined PDF
    RenderBook {
        book_root: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Path to a stylesheet TOML file. Falls back to `<book_root>/style.toml` if present,
        /// then to built-in defaults.
        #[arg(long)]
        style: Option<PathBuf>,
        /// Document title, available to header/footer templates as {title}. Overrides
        /// [document].title from the stylesheet if both are given.
        #[arg(long)]
        title: Option<String>,
        /// Document author, available to header/footer templates as {author}. Overrides
        /// [document].author from the stylesheet if both are given.
        #[arg(long)]
        author: Option<String>,
    },
}

/// Applies `--title`/`--author` on top of the stylesheet's own `[document]` section, in place --
/// the CLI flag wins if both are given, otherwise the stylesheet's value (including its default
/// of "") passes through unchanged.
fn apply_document_overrides(stylesheet: &mut md2pdf_style::Stylesheet, title: Option<String>, author: Option<String>) {
    if let Some(title) = title {
        stylesheet.document.title = title;
    }
    if let Some(author) = author {
        stylesheet.document.author = author;
    }
}

fn build_font_system(typography: &md2pdf_style::TypographyStyle) -> cosmic_text::FontSystem {
    let mut font_db = fontdb::Database::new();
    if typography.use_system_fonts {
        font_db.load_system_fonts();
    }
    for dir in &typography.font_dirs {
        font_db.load_fonts_dir(dir);
    }
    cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db)
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
        Commands::Render { input, output, style, title, author } => {
            let mut stylesheet =
                timed_stage("Resolving stylesheet", || md2pdf_style::Stylesheet::resolve(style.as_deref(), None))?;
            apply_document_overrides(&mut stylesheet, title, author);

            let markdown = std::fs::read_to_string(&input)?;
            let mut slugs = md2pdf_ast::SlugGenerator::new();
            let mut next_diagram_id = 0usize;
            let mut ast = timed_stage("Parsing markdown", || {
                md2pdf_ast::parse_with_style(&markdown, &mut slugs, &mut next_diagram_id, &stylesheet)
            });
            md2pdf_ast::tag_diagram_origins(&mut ast, &input);

            let highlighter = Highlighter::with_style(&stylesheet);
            let ast = timed_stage("Highlighting code blocks", || highlighter.highlight(ast));
            let diagrams = timed_stage("Compiling diagrams", || md2pdf_enrich::compile_diagrams(&ast));

            let base_dir = input.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();

            let mut font_system = timed_stage("Loading fonts", || build_font_system(&stylesheet.typography));

            let output_layout = timed_stage("Laying out pages", || {
                md2pdf_layout::layout_with_header_footer(&ast, &mut font_system, &base_dir, &diagrams, &stylesheet)
            });
            let pdf_bytes = timed_stage("Rendering PDF", || {
                md2pdf_pdf::render_pdf(&output_layout.pages, font_system.db(), &output_layout.images, &diagrams, &output_layout.anchors, output_layout.page_width_pt, output_layout.page_height_pt, &output_layout.toc_entries)
            })?;
            timed_stage("Writing output", || std::fs::write(&output, pdf_bytes))?;
            eprintln!("Wrote {} ({} pages)", output.display(), output_layout.pages.len());
            Ok(())
        }
        Commands::RenderBook { book_root, output, style, title, author } => {
            let mut stylesheet = timed_stage("Resolving stylesheet", || {
                md2pdf_style::Stylesheet::resolve(style.as_deref(), Some(&book_root))
            })?;
            apply_document_overrides(&mut stylesheet, title, author);

            let ast = timed_stage("Loading book", || md2pdf_book::load_book(&book_root, &stylesheet))?;

            let highlighter = Highlighter::with_style(&stylesheet);
            let ast = timed_stage("Highlighting code blocks", || highlighter.highlight(ast));
            let diagrams = timed_stage("Compiling diagrams", || md2pdf_enrich::compile_diagrams(&ast));

            let mut font_system = timed_stage("Loading fonts", || build_font_system(&stylesheet.typography));

            // Every embedded image path was already rewritten to absolute during load_book (each
            // chapter can live in a different subdirectory), so base_dir is never actually
            // joined onto anything -- but decode_images also uses it as a security boundary,
            // rejecting any absolute path that isn't one of its descendants. Passing "." there
            // (the CLI process's own CWD) silently dropped every image in any book that didn't
            // happen to live under the current directory; book_root is the real boundary.
            let output_layout = timed_stage("Laying out pages", || {
                md2pdf_layout::layout_with_header_footer(&ast, &mut font_system, &book_root, &diagrams, &stylesheet)
            });
            let pdf_bytes = timed_stage("Rendering PDF", || {
                md2pdf_pdf::render_pdf(&output_layout.pages, font_system.db(), &output_layout.images, &diagrams, &output_layout.anchors, output_layout.page_width_pt, output_layout.page_height_pt, &output_layout.toc_entries)
            })?;
            timed_stage("Writing output", || std::fs::write(&output, pdf_bytes))?;
            eprintln!("Wrote {} ({} pages)", output.display(), output_layout.pages.len());
            Ok(())
        }
    }
}
