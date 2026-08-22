use clap::{Parser, Subcommand};
use sardown_enrich::Highlighter;

mod bench;
mod benchgen;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sardown")]
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
        /// \[document\].title from the stylesheet if both are given.
        #[arg(long)]
        title: Option<String>,
        /// Document author, available to header/footer templates as {author}. Overrides
        /// \[document\].author from the stylesheet if both are given.
        #[arg(long)]
        author: Option<String>,
        /// Document date ("YYYY-MM-DD" or any other literal string), available to header/footer
        /// templates as {date}. Overrides \[document\].date from the stylesheet if both are given;
        /// if neither is given, defaults to today's date.
        #[arg(long)]
        date: Option<String>,
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
        /// \[document\].title from the stylesheet if both are given.
        #[arg(long)]
        title: Option<String>,
        /// Document author, available to header/footer templates as {author}. Overrides
        /// \[document\].author from the stylesheet if both are given.
        #[arg(long)]
        author: Option<String>,
        /// Document date ("YYYY-MM-DD" or any other literal string), available to header/footer
        /// templates as {date}. Overrides \[document\].date from the stylesheet if both are given;
        /// if neither is given, defaults to today's date.
        #[arg(long)]
        date: Option<String>,
    },
    /// Render a Markdown slide deck (split into slides on `---`) to PDF
    RenderSlides {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Path to a stylesheet TOML file. Falls back to built-in defaults if omitted.
        #[arg(long)]
        style: Option<PathBuf>,
        /// Document title, available to header/footer templates as {title}. Overrides
        /// \[document\].title from the stylesheet if both are given.
        #[arg(long)]
        title: Option<String>,
        /// Document author, available to header/footer templates as {author}. Overrides
        /// \[document\].author from the stylesheet if both are given.
        #[arg(long)]
        author: Option<String>,
        /// Document date ("YYYY-MM-DD" or any other literal string), available to header/footer
        /// templates as {date}. Overrides \[document\].date from the stylesheet if both are given;
        /// if neither is given, defaults to today's date.
        #[arg(long)]
        date: Option<String>,
    },
    /// Generate seeded complex Markdown input, render it, and report per-stage timings
    Bench {
        /// PRNG seed; the same seed regenerates byte-identical benchmark input
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// What to generate and which production pipeline to drive
        #[arg(long, value_enum, default_value_t = bench::BenchMode::Render)]
        mode: bench::BenchMode,
        /// Output volume: approximate page count (render/book) or slide count (slides)
        #[arg(long, default_value_t = 25)]
        pages: usize,
        /// Full-pipeline repetitions averaged into the timing report
        #[arg(long, default_value_t = 3)]
        iterations: usize,
        /// Path to a stylesheet TOML file passed through to the pipeline. Falls back to
        /// built-in defaults if omitted.
        #[arg(long)]
        style: Option<PathBuf>,
        /// Write the generated Markdown (render/slides modes) here for inspection or
        /// regression diffing
        #[arg(long)]
        markdown_out: Option<PathBuf>,
        /// Book mode: directory for the generated book tree. Defaults to a fresh directory
        /// under the system temp dir; an existing tree is replaced on each run.
        #[arg(long)]
        book_dir: Option<PathBuf>,
        /// Where to write the final rendered PDF. PDF bytes are discarded when omitted.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Applies `--title`/`--author`/`--date` on top of the stylesheet's own `[document]` section, in
/// place -- a CLI flag wins if both are given, otherwise the stylesheet's own value (including
/// its default of `""`) passes through unchanged. `date` is the one exception: an empty result
/// (neither the flag nor the stylesheet set one) falls back to today's date rather than staying
/// empty, since "no date was configured" should still show *something* sensible in a template.
fn apply_document_overrides(stylesheet: &mut sardown_style::Stylesheet, title: Option<String>, author: Option<String>, date: Option<String>) {
    if let Some(title) = title {
        stylesheet.document.title = title;
    }
    if let Some(author) = author {
        stylesheet.document.author = author;
    }
    if let Some(date) = date {
        stylesheet.document.date = date;
    } else if stylesheet.document.date.is_empty() {
        stylesheet.document.date = today_date_string();
    }
}

/// Today's date as "YYYY-MM-DD" (UTC), used to populate `{date}` when neither `--date` nor the
/// stylesheet's `[document].date` set it explicitly.
fn today_date_string() -> String {
    let days_since_epoch =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("system clock is before the Unix epoch").as_secs() as i64 / 86400;
    let (year, month, day) = civil_from_days(days_since_epoch);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's `civil_from_days`: converts a day count since 1970-01-01 (UTC) into a
/// proleptic-Gregorian (year, month, day) -- see
/// <http://howardhinnant.github.io/date_algorithms.html>. Avoids pulling in a date/time crate as a
/// real (non-dev) dependency for one ISO-8601 string; chrono is already in the dependency tree,
/// but only as a dev-dependency of the golden-image/PDF-rendering test tooling.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// The directory `input`'s own relative image references (and the SVG/security containment
/// check in `decode_images`) should resolve against. `Path::parent()` returns `Some("")` -- an
/// empty path, not `None` -- for a *bare* relative filename with no directory component (e.g.
/// "doc.md"), so a plain `.unwrap_or_else(|| Path::new("."))` never reaches its fallback in that
/// case: `PathBuf::from("")` fails to canonicalize at all ("No such file or directory"), silently
/// dropping every embedded image. Treating an empty parent the same as "no parent" avoids that.
fn base_dir_of(input: &std::path::Path) -> std::path::PathBuf {
    match input.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    }
}

fn build_font_system(typography: &sardown_style::TypographyStyle) -> cosmic_text::FontSystem {
    let mut font_db = fontdb::Database::new();
    if typography.use_system_fonts {
        font_db.load_system_fonts();
    }
    for dir in &typography.font_dirs {
        font_db.load_fonts_dir(dir);
    }
    cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db)
}

/// Prints `label`, runs `f`, then reports how long it took and whether it actually succeeded --
/// on stderr, so it never mixes with piped/redirected output. A large book's render has no other
/// feedback for several seconds at a time otherwise, which reads as a hang rather than progress.
/// Every stage is fallible (wrap an infallible one in `Ok(..)`) so a failing stage reports
/// "failed", not the same "done" a successful one gets -- printing "done" unconditionally would
/// misleadingly claim success for the exact stage whose own error is about to propagate.
fn timed_stage<T>(label: &str, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    eprint!("{label}... ");
    let start = std::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64();
    match &result {
        Ok(_) => eprintln!("done ({elapsed:.2}s)"),
        Err(_) => eprintln!("failed ({elapsed:.2}s)"),
    }
    result
}

/// Runs `Highlighter::with_style`'s highlight pass and Mermaid diagram compilation -- the two
/// enrichment stages every render mode except `render-slides` needs identically (`render_slide_deck`
/// runs its own copy of this same pipeline internally).
///
/// The syntect highlighter is only constructed when the document actually contains code blocks:
/// `with_style` loads every default syntax definition and the complete theme (well over a second
/// of work) and would do nothing for a document without any.
///
/// Takes the font system because diagram compilation parses each rendered Mermaid SVG against
/// the document's own font database (`sardown_enrich::svg_tree_options`) -- so the font system
/// must exist before diagrams compile, not just before layout.
fn highlight_and_compile_diagrams(
    ast: Vec<sardown_ast::BlockNode>,
    stylesheet: &sardown_style::Stylesheet,
    font_system: &cosmic_text::FontSystem,
) -> anyhow::Result<(Vec<sardown_ast::BlockNode>, sardown_enrich::DiagramTable)> {
    let ast = if sardown_enrich::ast_contains_code_block(&ast) {
        let highlighter = Highlighter::with_style(stylesheet);
        timed_stage("Highlighting code blocks", || Ok(highlighter.highlight(ast)))?
    } else {
        ast
    };
    let svg_options = sardown_enrich::svg_tree_options(font_system.db());
    let diagrams = timed_stage("Compiling diagrams", || Ok(sardown_enrich::compile_diagrams(&ast, &svg_options)))?;
    Ok((ast, diagrams))
}

/// Renders `output_layout` to PDF bytes and writes them to `output_path` -- the tail every render
/// mode shares identically once it has its own `LayoutOutput` in hand.
fn write_pdf_output(
    output_layout: &sardown_layout::LayoutOutput,
    font_system: &cosmic_text::FontSystem,
    output_path: &std::path::Path,
    item_noun: &str,
) -> anyhow::Result<()> {
    let pdf_bytes = timed_stage("Rendering PDF", || {
        sardown_pdf::render_pdf(
            &output_layout.pages,
            font_system.db(),
            &output_layout.images,
            &output_layout.diagrams,
            &output_layout.anchors,
            output_layout.page_width_pt,
            output_layout.page_height_pt,
            &output_layout.toc_entries,
        )
    })?;
    timed_stage("Writing output", || Ok(std::fs::write(output_path, pdf_bytes)?))?;
    eprintln!("Wrote {} ({} {item_noun})", output_path.display(), output_layout.pages.len());
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Render { input, output, style, title, author, date } => {
            let mut stylesheet = timed_stage("Resolving stylesheet", || sardown_style::Stylesheet::resolve(style.as_deref(), None))?;
            apply_document_overrides(&mut stylesheet, title, author, date);

            let markdown = std::fs::read_to_string(&input)?;
            let mut slugs = sardown_ast::SlugGenerator::new();
            let mut next_diagram_id = 0usize;
            let mut ast = timed_stage("Parsing markdown", || Ok(sardown_ast::parse_with_style(&markdown, &mut slugs, &mut next_diagram_id, &stylesheet)))?;
            sardown_ast::tag_diagram_origins(&mut ast, &input);

            // Fonts load before enrichment: Mermaid compilation parses each rendered SVG against
            // the document's own fontdb, so it needs the loaded font system in hand.
            let mut font_system = timed_stage("Loading fonts", || Ok(build_font_system(&stylesheet.typography)))?;

            let (ast, diagrams) = highlight_and_compile_diagrams(ast, &stylesheet, &font_system)?;

            let base_dir = base_dir_of(&input);

            let output_layout =
                timed_stage("Laying out pages", || Ok(sardown_layout::layout_with_header_footer(&ast, &mut font_system, &base_dir, &diagrams, &stylesheet)))?;
            write_pdf_output(&output_layout, &font_system, &output, "pages")
        }
        Commands::RenderBook { book_root, output, style, title, author, date } => {
            let mut stylesheet = timed_stage("Resolving stylesheet", || sardown_style::Stylesheet::resolve(style.as_deref(), Some(&book_root)))?;
            apply_document_overrides(&mut stylesheet, title, author, date);

            let ast = timed_stage("Loading book", || sardown_book::load_book(&book_root, &stylesheet))?;

            // As in Commands::Render: fonts before enrichment, because diagram compilation
            // parses against the document's own fontdb.
            let mut font_system = timed_stage("Loading fonts", || Ok(build_font_system(&stylesheet.typography)))?;

            let (ast, diagrams) = highlight_and_compile_diagrams(ast, &stylesheet, &font_system)?;

            // Every embedded image path was already rewritten to absolute during load_book (each
            // chapter can live in a different subdirectory), so base_dir is never actually
            // joined onto anything -- but decode_images also uses it as a security boundary,
            // rejecting any absolute path that isn't one of its descendants. Passing "." there
            // (the CLI process's own CWD) silently dropped every image in any book that didn't
            // happen to live under the current directory; book_root is the real boundary.
            let output_layout =
                timed_stage("Laying out pages", || Ok(sardown_layout::layout_with_header_footer(&ast, &mut font_system, &book_root, &diagrams, &stylesheet)))?;
            write_pdf_output(&output_layout, &font_system, &output, "pages")
        }
        Commands::RenderSlides { input, output, style, title, author, date } => {
            let mut stylesheet = timed_stage("Resolving stylesheet", || sardown_style::Stylesheet::resolve(style.as_deref(), None))?;
            apply_document_overrides(&mut stylesheet, title, author, date);

            let markdown = std::fs::read_to_string(&input)?;
            let base_dir = base_dir_of(&input);
            let mut font_system = timed_stage("Loading fonts", || Ok(build_font_system(&stylesheet.typography)))?;

            let output_layout =
                timed_stage("Laying out slides", || sardown_slides::render_slide_deck(&markdown, &input, &base_dir, &mut font_system, &stylesheet))?;
            write_pdf_output(&output_layout, &font_system, &output, "slides")
        }
        Commands::Bench { seed, mode, pages, iterations, style, markdown_out, book_dir, output } => {
            crate::bench::run(bench::BenchArgs { seed, mode, pages, iterations, style, markdown_out, book_dir, output })
        }
    }
}

#[cfg(test)]
mod date_tests {
    use super::civil_from_days;

    #[test]
    fn known_epoch_days_convert_to_the_correct_calendar_date() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        assert_eq!(civil_from_days(365), (1971, 1, 1)); // 1970 is not a leap year
        assert_eq!(civil_from_days(10957), (2000, 1, 1));
        assert_eq!(civil_from_days(19723), (2024, 1, 1));
        assert_eq!(civil_from_days(20089), (2025, 1, 1));
    }
}
