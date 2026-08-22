//! `sardown bench` runner: generates seeded input, drives the production pipeline the chosen
//! number of times, and reports per-stage min/mean/max timings.
//!
//! Generation happens once per invocation (it is deterministic in the seed and not interesting
//! to time); the font system is also built once and reused across iterations so the table
//! reflects steady-state rendering rather than first-touch disk I/O. Every other stage mirrors
//! what the corresponding production subcommand does, in the same order.

use anyhow::Context;
use sardown_enrich::Highlighter;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What to generate and which production pipeline to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum BenchMode {
    /// One flowing document through render's pipeline
    Render,
    /// An mdBook source tree through render-book's pipeline
    Book,
    /// A `---`-split deck through render-slides' pipeline
    Slides,
}

pub struct BenchArgs {
    pub seed: u64,
    pub mode: BenchMode,
    pub pages: usize,
    pub iterations: usize,
    pub style: Option<PathBuf>,
    pub markdown_out: Option<PathBuf>,
    pub book_dir: Option<PathBuf>,
    pub output: Option<PathBuf>,
}

/// Per-stage samples across iterations; BTreeMap keeps stage rows in a stable order.
struct Timings {
    samples: BTreeMap<&'static str, Vec<Duration>>,
}

impl Timings {
    fn new() -> Self {
        Self { samples: BTreeMap::new() }
    }

    fn time<T>(&mut self, stage: &'static str, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
        let start = Instant::now();
        let out = f()?;
        self.samples.entry(stage).or_default().push(start.elapsed());
        Ok(out)
    }

    fn report(&self) {
        println!("\n{:<24} {:>9} {:>9} {:>9}", "stage", "min", "mean", "max");
        for (stage, runs) in &self.samples {
            let secs = |d: Duration| d.as_secs_f64();
            let min = secs(*runs.iter().min().unwrap());
            let max = secs(*runs.iter().max().unwrap());
            let mean = runs.iter().map(|d| d.as_secs_f64()).sum::<f64>() / runs.len() as f64;
            println!("{stage:<24} {min:>8.3}s {mean:>8.3}s {max:>8.3}s");
        }
    }
}

fn fmt_bytes(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MiB", n as f64 / 1024.0 / 1024.0)
    } else if n >= 1024 {
        format!("{:.1} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

fn emit_pdf(t: &mut Timings, font_system: &cosmic_text::FontSystem, output: &sardown_layout::LayoutOutput) -> anyhow::Result<Vec<u8>> {
    t.time("rendering pdf", || {
        sardown_pdf::render_pdf(
            &output.pages,
            font_system.db(),
            &output.images,
            &output.diagrams,
            &output.anchors,
            output.page_width_pt,
            output.page_height_pt,
            &output.toc_entries,
        )
    })
}

/// Entry point for `sardown bench`. Generation is untimed; each iteration re-runs the full
/// production pipeline for the selected mode.
pub fn run(args: BenchArgs) -> anyhow::Result<()> {
    if args.iterations == 0 {
        anyhow::bail!("--iterations must be at least 1");
    }
    println!("mode: {:?} | seed: {} | volume target: {} | iterations: {}", args.mode, args.seed, args.pages, args.iterations);

    // ---- generation (untimed, deterministic in the seed) ----
    let stats_line;
    let mut markdown: Option<String> = None;
    let mut book_root: Option<PathBuf> = None;
    match args.mode {
        BenchMode::Render => {
            let doc = crate::benchgen::generate_document(args.seed, args.pages);
            stats_line = doc.stats.summary();
            markdown = Some(doc.markdown);
        }
        BenchMode::Slides => {
            let deck = crate::benchgen::generate_deck(args.seed, args.pages);
            stats_line = deck.stats.summary();
            markdown = Some(deck.markdown);
        }
        BenchMode::Book => {
            let chapters = (args.pages / 5).clamp(3, 12);
            let tree = crate::benchgen::generate_book_tree(args.seed, chapters);
            stats_line = tree.stats.summary();
            let root = args.book_dir.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("sardown-bench-book-{}", args.seed)));
            if root.exists() {
                std::fs::remove_dir_all(&root).context("clearing previous bench book dir")?;
            }
            for (rel, contents) in &tree.files {
                let path = root.join(rel);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
                }
                std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
            }
            println!("book tree: {} files under {}", tree.files.len(), root.display());
            book_root = Some(root);
        }
    }
    if let Some(path) = &args.markdown_out {
        if let Some(md) = &markdown {
            std::fs::write(path, md).with_context(|| format!("writing generated markdown to {}", path.display()))?;
            println!("markdown written: {} ({})", path.display(), fmt_bytes(md.len()));
        } else {
            println!("note: --markdown-out applies to render/slides modes; book mode's source is the generated tree");
        }
    }
    println!("{stats_line}");

    // ---- shared setup (untimed): fonts are reused across iterations ----
    let style_fallback: Option<&Path> = book_root.as_deref();
    let mut stylesheet = sardown_style::Stylesheet::resolve(args.style.as_deref(), style_fallback)?;
    // A deck benchmark has to look like a deck: the built-in default page is portrait
    // Letter, which would render the generated deck as a stack of portrait text pages.
    // Unless the user brought their own stylesheet, overlay a 16:9 slide page with
    // slide-appropriate margins.
    if args.mode == BenchMode::Slides && args.style.is_none() {
        stylesheet.page.width_mm = Some(338.667);
        stylesheet.page.height_mm = Some(190.5);
        stylesheet.page.margin_mm = 14.0;
    }
    let mut font_system = crate::build_font_system(&stylesheet.typography);

    let mut t = Timings::new();
    let mut last_pdf: Vec<u8> = Vec::new();
    let mut last_pages = 0usize;
    let mut source_bytes = 0usize;

    for _ in 0..args.iterations {
        let output = match args.mode {
            BenchMode::Render => {
                let md = markdown.as_deref().expect("render mode builds markdown");
                source_bytes = md.len();
                let mut slugs = sardown_ast::SlugGenerator::new();
                let mut next_diagram_id = 0usize;
                let mut ast = t.time("parsing markdown", || Ok(sardown_ast::parse_with_style(md, &mut slugs, &mut next_diagram_id, &stylesheet)))?;
                sardown_ast::tag_diagram_origins(&mut ast, Path::new("bench.md"));
                let ast = t.time("highlighting code blocks", || {
                    Ok(if sardown_enrich::ast_contains_code_block(&ast) { Highlighter::with_style(&stylesheet).highlight(ast) } else { ast })
                })?;
                let diagrams = t.time("compiling diagrams", || {
                    let options = sardown_enrich::svg_tree_options(font_system.db());
                    Ok(sardown_enrich::compile_diagrams(&ast, &options))
                })?;
                let out = t.time("laying out pages", || {
                    Ok(sardown_layout::layout_with_header_footer(&ast, &mut font_system, Path::new("."), &diagrams, &stylesheet))
                })?;
                let pdf = emit_pdf(&mut t, &font_system, &out)?;
                (out, pdf)
            }
            BenchMode::Book => {
                let root = book_root.clone().expect("book mode builds a tree");
                let ast = t.time("loading book", || sardown_book::load_book(&root, &stylesheet))?;
                source_bytes = std::fs::read_to_string(root.join("src").join("SUMMARY.md")).map(|s| s.len()).unwrap_or(0);
                let ast = t.time("highlighting code blocks", || {
                    Ok(if sardown_enrich::ast_contains_code_block(&ast) { Highlighter::with_style(&stylesheet).highlight(ast) } else { ast })
                })?;
                let diagrams = t.time("compiling diagrams", || {
                    let options = sardown_enrich::svg_tree_options(font_system.db());
                    Ok(sardown_enrich::compile_diagrams(&ast, &options))
                })?;
                let out =
                    t.time("laying out pages", || Ok(sardown_layout::layout_with_header_footer(&ast, &mut font_system, &root, &diagrams, &stylesheet)))?;
                let pdf = emit_pdf(&mut t, &font_system, &out)?;
                (out, pdf)
            }
            BenchMode::Slides => {
                let md = markdown.as_deref().expect("slides mode builds a deck");
                source_bytes = md.len();
                let out = t.time("slides pipeline", || {
                    sardown_slides::render_slide_deck(md, Path::new("bench-deck.md"), Path::new("."), &mut font_system, &stylesheet)
                })?;
                let pdf = emit_pdf(&mut t, &font_system, &out)?;
                (out, pdf)
            }
        };
        last_pages = output.0.pages.len();
        last_pdf = output.1;
    }

    t.report();
    println!("\nsource: {} | pages/slides rendered: {} | final pdf: {}", fmt_bytes(source_bytes), last_pages, fmt_bytes(last_pdf.len()));
    if let Some(path) = &args.output {
        std::fs::write(path, &last_pdf).with_context(|| format!("writing PDF to {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
