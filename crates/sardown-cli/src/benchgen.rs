//! Deterministic content generator behind `sardown bench`.
//!
//! Everything here is pure: the same seed always produces byte-identical output, so benchmark
//! runs are comparable across builds and a generated document can be regenerated exactly for
//! inspection or regression diffing. The PRNG is a hand-rolled `xorshift64*` -- good enough for
//! text/shape variety, tiny, and dependency-free.
//!
//! Three generators share one vocabulary and one set of section builders:
//! - [`generate_document`] -- a single flowing document exercising every feature reachable from
//!   one Markdown file (headings h1-h6, inline styles, links internal + external, ordered /
//!   nested / task lists, nested blockquotes, GFM tables with wrapping cells, fenced code in
//!   several languages including an over-long line, thematic breaks, PNG and SVG data-URI
//!   images, Mermaid flowchart + sequence diagrams, `::columns` groups).
//! - [`generate_deck`] -- a `---`-split slide deck (dense slides that exercise auto-shrink,
//!   code, columns, table, quote, and image slides). No `@layout:` directives: decks render
//!   against the built-in default layout without needing a custom stylesheet.
//! - [`generate_book_tree`] -- an mdBook source tree (`book.toml`, `src/SUMMARY.md` with a
//!   prefix chapter and a nested sub-chapter, chapter files with cross-file links into each
//!   other's headings, and a shared snippet pulled in via `{{#include}}`).
//!
//! Not expressible in generated input (and therefore deliberately absent): `PageBreak` is only
//! ever produced by `render-book` between chapters, never by Markdown.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

/// Counts of every feature a generated input contains, printed by `sardown bench` so coverage
/// is auditable per run instead of taken on faith.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GenStats {
    pub headings: usize,
    pub paragraphs: usize,
    pub lists: usize,
    pub task_list_items: usize,
    pub blockquotes: usize,
    pub tables: usize,
    pub code_blocks: usize,
    pub thematic_breaks: usize,
    pub png_images: usize,
    pub svg_images: usize,
    pub mermaid_diagrams: usize,
    pub column_groups: usize,
    pub internal_links: usize,
    pub external_links: usize,
}

impl GenStats {
    /// Compact summary lines for the bench report.
    pub fn summary(&self) -> String {
        format!(
            "coverage: headings={} paragraphs={} lists={} tasks={} quotes={} tables={} code={} hr={} \
             png={} svg={} mermaid={} columns={} links(int/ext)={}/{}",
            self.headings,
            self.paragraphs,
            self.lists,
            self.task_list_items,
            self.blockquotes,
            self.tables,
            self.code_blocks,
            self.thematic_breaks,
            self.png_images,
            self.svg_images,
            self.mermaid_diagrams,
            self.column_groups,
            self.internal_links,
            self.external_links,
        )
    }
}

/// A generated single Markdown document plus its coverage counts.
pub struct GeneratedDoc {
    pub markdown: String,
    pub stats: GenStats,
}

/// A generated book tree: `(path relative to the book root, file contents)` pairs. Always
/// contains `book.toml`, `src/SUMMARY.md`, the chapter files it names, and one include snippet.
pub struct GeneratedBook {
    pub files: Vec<(String, String)>,
    pub stats: GenStats,
}

// ---------------------------------------------------------------------------------------------
// Seeded PRNG: xorshift64*. Deterministic across platforms (pure integer arithmetic).
// ---------------------------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Zero is xorshift's fixed point; the `| 1` guarantees a nonzero state for any seed.
        Self((seed ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

// ---------------------------------------------------------------------------------------------
// Vocabulary and section builders (each builder owns its own stat counting)
// ---------------------------------------------------------------------------------------------

const WORDS: &[&str] = &[
    "the",
    "quick",
    "brown",
    "fox",
    "jumps",
    "over",
    "lazy",
    "dog",
    "while",
    "typesetting",
    "engines",
    "shape",
    "text",
    "into",
    "positioned",
    "glyph",
    "runs",
    "before",
    "pagination",
    "decides",
    "where",
    "each",
    "line",
    "lands",
    "shaping",
    "kerning",
    "ligatures",
    "hyphenation",
    "justification",
    "subsetting",
    "outline",
    "annotation",
    "destination",
    "anchor",
    "column",
];

const SENTENCES: &[&str] = &[
    "Shaping turns character clusters into positioned glyph runs before anything reaches a page.",
    "Pagination walks shaped lines until the next one would cross the bottom margin.",
    "Font fallback walks every loaded face per word when the primary face misses a codepoint.",
    "Tables measure their longest cell once and reuse the shaped runs at placement time.",
    "Header and footer zones shape once per distinct resolved text, not once per page.",
    "Syntax highlighting loads its grammar set only when a code fence actually exists.",
    "Diagrams compile to SVG once per document and are re-emitted from cached trees.",
    "Hyphenation inserts literal hyphens where the dictionary allows a break point.",
    "Column groups lay out in isolation and shift back into the page's own coordinates.",
    "Anchors recorded during layout become link annotations and outline destinations.",
];

const CODE_LANGS: &[&str] = &["rust", "python", "json", "toml"];

fn code_sample(lang: &str) -> &'static str {
    match lang {
        "rust" => "fn process(input: &str) -> Vec<String> {\n    input.split_whitespace().map(str::to_owned).collect()\n}",
        "python" => "def process(text):\n    return [word for word in text.split() if word.isalpha()]",
        "json" => "{\n  \"mode\": \"bench\",\n  \"features\": [\"tables\", \"lists\", \"code\"]\n}",
        _ => "[document]\ntitle = \"Generated\"\nauthors = [\"bench\"]",
    }
}

fn words(rng: &mut Rng, count: usize) -> String {
    (0..count).map(|_| *rng.pick(WORDS)).collect::<Vec<_>>().join(" ")
}

fn paragraph(rng: &mut Rng, stats: &mut GenStats, sentences: usize) -> String {
    stats.paragraphs += 1;
    (0..sentences)
        .map(|_| {
            if rng.chance(30) {
                let base = *rng.pick(SENTENCES);
                match rng.below(5) {
                    0 => format!("This passage is **bold on record**: {base}"),
                    1 => format!("A *slanted aside* follows: {base}"),
                    2 => format!("Struck ~~from the record~~ yet kept: {base}"),
                    3 => format!("The knob `max_width_pt` matters here. {base}"),
                    _ => base.to_string(),
                }
            } else {
                let n = rng.range(8, 16);
                words(rng, n)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A small deterministic RGBA pattern image, PNG-encoded. Same seed and dimensions produce
/// identical bytes, so generated documents are reproducible end to end.
fn png_data_uri(rng: &mut Rng, width: u32, height: u32, stats: &mut GenStats) -> String {
    use image::codecs::png::PngEncoder;
    use image::{ColorType, ImageEncoder, RgbaImage};

    let mut img = RgbaImage::new(width, height);
    let a = rng.next_u64() as u8;
    let b = rng.next_u64() as u8;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let checker = ((x / 8) ^ (y / 8)) % 2 == 0;
        let (r, g, bl) = if checker { (a, b, 255 - a) } else { (b.wrapping_add(x as u8), a, y as u8) };
        *pixel = image::Rgba([r, g, bl, 255]);
    }
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded).write_image(img.as_raw(), width, height, ColorType::Rgba8.into()).expect("PNG encoding of an in-memory buffer cannot fail");
    stats.png_images += 1;
    format!("data:image/png;base64,{}", BASE64.encode(encoded))
}

fn svg_data_uri(rng: &mut Rng, stats: &mut GenStats) -> String {
    let hue = rng.below(360);
    let r = rng.below(40) + 10;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"80\" viewBox=\"0 0 120 80\">\
         <rect width=\"120\" height=\"80\" fill=\"hsl({hue},60%,85%)\"/>\
         <circle cx=\"35\" cy=\"40\" r=\"{r}\" fill=\"hsl({hue},70%,45%)\"/>\
         <rect x=\"70\" y=\"20\" width=\"36\" height=\"40\" fill=\"hsl({},70%,55%)\"/>\
         <text x=\"8\" y=\"74\" font-family=\"sans-serif\" font-size=\"10\">bench</text></svg>",
        (hue + 140) % 360,
    );
    stats.svg_images += 1;
    format!("data:image/svg+xml;base64,{}", BASE64.encode(svg.as_bytes()))
}

fn mermaid_source(kind: usize, tag: usize) -> String {
    match kind % 2 {
        0 => format!("flowchart TD\n    A[Bench {tag}] --> B{{fits?}}\n    B -->|yes| C[Emit]\n    B -->|no| D[Shrink]\n    D --> B"),
        _ => format!("sequenceDiagram\n    participant U as User\n    participant S as Server\n    U->>S: GET /bench/{tag}\n    S-->>U: 200 OK"),
    }
}

/// One `::columns` group with two columns of seeded filler.
fn columns_block(rng: &mut Rng, stats: &mut GenStats, tag: usize) -> String {
    stats.column_groups += 1;
    let left = paragraph(rng, stats, 2);
    let right_items: Vec<String> = (0..rng.range(3, 5)).map(|i| format!("- item {tag}.{i} {}", rng.pick(WORDS))).collect();
    format!("::columns\n\n::column\n\n{left}\n\n::column\n\n**Column {tag}**\n\n{}\n\n::end\n", right_items.join("\n"))
}

fn table_block(rng: &mut Rng, stats: &mut GenStats, rows: usize, cols: usize, tag: usize) -> String {
    stats.tables += 1;
    let header: Vec<String> = (0..cols).map(|c| format!("Metric {tag}.{c}")).collect();
    let mut out = format!("| {} |\n|{}|\n", header.join(" | "), vec!["---"; cols].join("|"));
    for r in 0..rows {
        let cells: Vec<String> = (0..cols)
            .map(|c| {
                if c == 0 {
                    format!("**row {r}**")
                } else if rng.chance(25) {
                    format!("`value_{r}_{c}`")
                } else {
                    let n = rng.range(2, 7);
                    words(rng, n)
                }
            })
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
    out
}

enum ListKind {
    Ordered,
    Bulleted,
    Tasks,
}

fn list_block(rng: &mut Rng, stats: &mut GenStats, kind: ListKind, tag: usize) -> String {
    stats.lists += 1;
    let n = rng.range(3, 6);
    match kind {
        ListKind::Ordered => {
            let start = rng.range(1, 9);
            (0..n).map(|i| format!("{}. ordered item {tag}.{}", start + i, rng.pick(WORDS))).collect::<Vec<_>>().join("\n")
        }
        ListKind::Bulleted => {
            let mut lines = Vec::new();
            for i in 0..n {
                lines.push(format!("- bullet {tag}.{i} {}", rng.pick(WORDS)));
                if i == 1 {
                    // Nested second level under the middle item.
                    for j in 0..2 {
                        lines.push(format!("  - nested {tag}.{i}.{j}"));
                    }
                }
            }
            lines.join("\n")
        }
        ListKind::Tasks => {
            stats.task_list_items += n;
            (0..n).map(|i| format!("- [{}] task {tag}.{i}", if rng.chance(50) { "x" } else { " " })).collect::<Vec<_>>().join("\n")
        }
    }
}

fn blockquote_block(rng: &mut Rng, stats: &mut GenStats, tag: usize) -> String {
    stats.blockquotes += 1;
    let inner = paragraph(rng, stats, 1);
    format!("> Quoted {tag}: {}\n>\n> > Nested quote depth two: {}\n", rng.pick(SENTENCES), inner)
}

// ---------------------------------------------------------------------------------------------
// Single-document generator
// ---------------------------------------------------------------------------------------------

/// Generates one flowing document. Every feature appears at least once regardless of seed;
/// `target_pages` scales how many randomized sections are added around that guaranteed core.
pub fn generate_document(seed: u64, target_pages: usize) -> GeneratedDoc {
    let mut rng = Rng::new(seed);
    let mut st = GenStats::default();
    // Calibrated against rendered output: one randomized section (a single table, paragraph,
    // or code block plus its heading) fills about a third of an A4 page, so three sections
    // land the document near the requested page count.
    let sections = (target_pages * 3).clamp(4, 1800);

    let mut out = String::with_capacity(64 * 1024);
    st.headings += 1;
    out.push_str(&format!("# Bench Document (seed {seed})\n\n"));

    // Guaranteed-coverage core, in a stable order so anchors exist before links reference them.
    st.internal_links += 1;
    st.external_links += 1;
    out.push_str(&format!(
        "{} An external link to the [sardown repository](https://github.com/abbra/sardown) and an\n\
         internal one to the [conclusion](#conclusion) appear in this opening paragraph.\n\n",
        paragraph(&mut rng, &mut st, 2)
    ));

    st.headings += 1;
    out.push_str("## Lists\n\n");
    out.push_str(&list_block(&mut rng, &mut st, ListKind::Bulleted, 1));
    out.push('\n');
    out.push_str(&list_block(&mut rng, &mut st, ListKind::Ordered, 2));
    out.push('\n');
    out.push_str(&list_block(&mut rng, &mut st, ListKind::Tasks, 3));
    out.push_str("\n\n");

    st.headings += 1;
    out.push_str("## Tables\n\n");
    out.push_str(&table_block(&mut rng, &mut st, 5, 4, 1));
    out.push('\n');

    st.headings += 1;
    out.push_str("## Code\n\n");
    for lang in CODE_LANGS {
        st.code_blocks += 1;
        out.push_str(&format!("```{lang}\n{}\n```\n\n", code_sample(lang)));
    }
    // Over-long single line: exercises shrink-to-fit when the stylesheet enables it.
    st.code_blocks += 1;
    let long_line: String = (0..160).map(|i| (b'a' + (i % 26)) as char).collect();
    out.push_str(&format!("```text\n{long_line}\n```\n\n"));

    st.headings += 1;
    out.push_str("---\n\n"); // guaranteed thematic break for every seed
    st.thematic_breaks += 1;
    out.push_str("## Quotes\n\n");
    out.push_str(&blockquote_block(&mut rng, &mut st, 1));
    out.push('\n');
    out.push_str(&blockquote_block(&mut rng, &mut st, 2));

    st.headings += 1;
    out.push_str("## Images\n\n");
    out.push_str(&format!(
        "![generated pattern]({})\n\n![generated diagram]({})\n\n",
        png_data_uri(&mut rng, 96, 64, &mut st),
        svg_data_uri(&mut rng, &mut st)
    ));

    st.headings += 1;
    out.push_str("## Diagrams\n\n");
    for i in 0..2 {
        st.mermaid_diagrams += 1;
        out.push_str(&format!("```mermaid\n{}\n```\n\n", mermaid_source(i, i)));
    }

    st.headings += 1;
    out.push_str("## Columns\n\n");
    out.push_str(&columns_block(&mut rng, &mut st, 1));
    out.push('\n');

    // Deep heading levels h4-h6 appear once, guaranteed.
    st.headings += 3;
    out.push_str("#### Level four heading\n\n##### Level five heading\n\n###### Level six heading\n\n");

    // Randomized sections: prose, tables, code, quotes, images, columns, breaks in seeded mixes.
    let kinds = ["prose", "table", "code", "quote", "image", "columns", "tasks"];
    for i in 0..sections {
        if i > 0 && rng.chance(20) {
            st.thematic_breaks += 1;
            out.push_str("---\n\n");
        }
        st.headings += 1;
        out.push_str(&format!("## Section {i}: {}\n\n", rng.pick(WORDS)));
        match *rng.pick(&kinds) {
            "prose" => {
                let n = rng.range(2, 4);
                out.push_str(&paragraph(&mut rng, &mut st, n));
                out.push('\n');
            }
            "table" => {
                let rows = rng.range(3, 9);
                let cols = rng.range(2, 6);
                out.push_str(&table_block(&mut rng, &mut st, rows, cols, i));
            }
            "code" => {
                st.code_blocks += 1;
                let lang = *rng.pick(CODE_LANGS);
                out.push_str(&format!("```{lang}\n{}\n```\n\n", code_sample(lang)));
            }
            "quote" => out.push_str(&blockquote_block(&mut rng, &mut st, i)),
            "image" => {
                let w = rng.range(48, 128) as u32;
                out.push_str(&format!("![section image]({})\n\n", png_data_uri(&mut rng, w, 64, &mut st)));
            }
            "columns" => out.push_str(&columns_block(&mut rng, &mut st, i)),
            "tasks" => out.push_str(&list_block(&mut rng, &mut st, ListKind::Tasks, i)),
            _ => unreachable!("kind list is closed"),
        }
        out.push('\n');
    }

    st.headings += 1;
    st.internal_links += 1;
    out.push_str("## Conclusion\n\n");
    out.push_str(&format!("Back to the [top of the lists section](#lists) one last time. {}\n", paragraph(&mut rng, &mut st, 1)));

    GeneratedDoc { markdown: out, stats: st }
}

// ---------------------------------------------------------------------------------------------
// Slide-deck generator
// ---------------------------------------------------------------------------------------------
// Slide-deck generator
// ---------------------------------------------------------------------------------------------

/// Generates a `---`-split deck that actually looks like a deck: a title slide, then one
/// compact block per slide with short bullet phrasing. A slice of slides is deliberately
/// overfull so auto-shrink does real work. No `@layout:` directives are emitted -- decks
/// render against the built-in default layout without needing a custom stylesheet.
pub fn generate_deck(seed: u64, target_slides: usize) -> GeneratedDoc {
    let mut rng = Rng::new(seed ^ 0xDEC0_05E5);
    let mut st = GenStats::default();
    let slides = target_slides.clamp(3, 60);

    let mut out = String::new();
    st.headings += 1;
    st.paragraphs += 1;
    out.push_str(&format!("# Bench Deck (seed {seed})\n\nAutomatically generated benchmark deck covering every slide-level feature.\n"));

    // (kind, dense?) pairs; dense bullet slides intentionally overflow so auto-shrink steps
    // down, the rest fit at full size. The cycle guarantees every block kind appears within
    // the first six content slides regardless of deck length.
    let kinds =
        [("bullets", false), ("code", false), ("bullets", true), ("columns", false), ("image", false), ("table", false), ("quote", false), ("tasks", false)];
    for i in 1..slides {
        let (kind, dense) = kinds[i % kinds.len()];
        out.push_str("\n---\n\n");
        st.headings += 1;
        out.push_str(&format!("## Slide {i}: {}\n\n", rng.pick(WORDS)));
        match kind {
            "bullets" => {
                st.lists += 1;
                let n = if dense { rng.range(11, 15) } else { rng.range(4, 7) };
                for j in 0..n {
                    if dense {
                        out.push_str(&format!("- point {i}.{j}: {}\n", rng.pick(SENTENCES)));
                    } else {
                        out.push_str(&format!("- {} {}\n", rng.pick(WORDS), rng.pick(WORDS)));
                    }
                }
                out.push('\n');
            }
            "code" => {
                st.code_blocks += 1;
                let lang = *rng.pick(CODE_LANGS);
                out.push_str(&format!("```{lang}\n{}\n```\n\n", code_sample(lang)));
            }
            "columns" => out.push_str(&columns_block(&mut rng, &mut st, i)),
            "table" => {
                let rows = rng.range(3, 5);
                out.push_str(&table_block(&mut rng, &mut st, rows, 3, i));
            }
            "quote" => {
                st.blockquotes += 1;
                out.push_str(&format!("> {}\n", rng.pick(SENTENCES)));
            }
            "image" => {
                out.push_str(&format!("![deck image]({})\n\n", png_data_uri(&mut rng, 96, 64, &mut st)));
            }
            "tasks" => out.push_str(&list_block(&mut rng, &mut st, ListKind::Tasks, i)),
            _ => unreachable!("kind list is closed"),
        }
    }

    GeneratedDoc { markdown: out, stats: st }
}

fn chapter_file_name(index_zero_based: usize) -> String {
    format!("chapter-{:02}", index_zero_based + 1)
}

pub fn generate_book_tree(seed: u64, chapter_count: usize) -> GeneratedBook {
    let mut rng = Rng::new(seed ^ 0xB00C_5EED);
    let chapters = chapter_count.clamp(3, 12);
    let has_sub_chapter = chapters >= 4;
    let mut st = GenStats::default();

    let mut files: Vec<(String, String)> = Vec::new();
    files.push(("book.toml".to_string(), "[book]\ntitle = \"Bench Book\"\nauthors = [\"sardown bench\"]\n".to_string()));

    let titles: Vec<String> = (0..chapters).map(|i| format!("Chapter {:02}", i + 1)).collect();

    let mut summary = String::from("# Summary\n\n[Introduction](introduction.md)\n\n");
    for (i, title) in titles.iter().enumerate() {
        summary.push_str(&format!("- [{title}]({}.md)\n", chapter_file_name(i)));
        if i == 1 && has_sub_chapter {
            summary.push_str("  - [Deep Dive](deep-dive.md)\n");
        }
    }
    files.push(("src/SUMMARY.md".to_string(), summary));

    // Prefix chapter: bare link paragraph outside the numbered list.
    files.push((
        "src/introduction.md".to_string(),
        format!("{}\n\nThis prefix chapter exists so the summary parser's prefix-chapter path runs.\n", paragraph(&mut rng, &mut st, 2)),
    ));

    // Shared snippet, included by chapter one via {{#include}}.
    files.push(("src/includes/snippet.md".to_string(), "- shared snippet item one\n- shared snippet item two\n".to_string()));

    for i in 0..chapters {
        let mut body = String::new();
        st.headings += 1; // the synthesized H1 comes from SUMMARY's title at combine time
        body.push_str(&format!("## Overview\n\n{}\n\n", paragraph(&mut rng, &mut st, 2)));

        if i == 0 {
            body.push_str("Included content follows:\n\n{{#include includes/snippet.md}}\n\n");
            body.push_str(&format!("![chapter image]({})\n\n", png_data_uri(&mut rng, 80, 56, &mut st)));
        }
        if i == 1 && has_sub_chapter {
            st.mermaid_diagrams += 1;
            body.push_str(&format!("```mermaid\n{}\n```\n\n", mermaid_source(0, i)));
        }

        let kind = if rng.chance(50) { ListKind::Ordered } else { ListKind::Bulleted };
        body.push_str(&list_block(&mut rng, &mut st, kind, i));
        body.push('\n');
        let rows = rng.range(3, 6);
        body.push_str(&table_block(&mut rng, &mut st, rows, 3, i));

        // Cross-file link into the next chapter's Overview anchor (the last wraps to chapter 1).
        let next_idx = (i + 1) % chapters;
        st.internal_links += 1;
        body.push_str(&format!("\nContinue reading [{}]({}.md#overview).\n", titles[next_idx], chapter_file_name(next_idx)));

        files.push((format!("src/{}.md", chapter_file_name(i)), body));
    }

    if has_sub_chapter {
        st.paragraphs += 1;
        files.push(("src/deep-dive.md".to_string(), format!("## Deep Dive\n\n{}\n", paragraph(&mut rng, &mut st, 2))));
    }

    GeneratedBook { files, stats: st }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_identical_documents() {
        let a = generate_document(1234, 8);
        let b = generate_document(1234, 8);
        assert_eq!(a.markdown, b.markdown);
        assert_eq!(a.stats, b.stats);
    }

    #[test]
    fn different_seeds_produce_different_documents() {
        let a = generate_document(1, 8);
        let b = generate_document(2, 8);
        assert_ne!(a.markdown, b.markdown);
    }

    #[test]
    fn document_covers_every_markdown_reachable_feature() {
        let doc = generate_document(7, 8);
        // These are structural guarantees of generate_document, not seed accidents: the
        // guaranteed-coverage core emits at least one of every feature before any
        // seed-randomized section is added.
        assert!(doc.stats.headings >= 10);
        assert!(doc.stats.paragraphs >= 3);
        assert!(doc.stats.lists >= 3 && doc.stats.task_list_items >= 3);
        assert!(doc.stats.tables >= 1 && doc.stats.code_blocks >= 5);
        assert!(doc.stats.blockquotes >= 2 && doc.stats.thematic_breaks >= 1);
        assert!(doc.stats.png_images >= 1 && doc.stats.svg_images >= 1);
        assert!(doc.stats.mermaid_diagrams >= 2 && doc.stats.column_groups >= 1);
        assert!(doc.stats.internal_links >= 2 && doc.stats.external_links >= 1);
        assert!(doc.markdown.contains("```mermaid"));
        assert!(doc.markdown.contains("::columns"));
        assert!(doc.markdown.contains("data:image/png;base64,"));
        assert!(doc.markdown.contains("data:image/svg+xml;base64,"));
        assert!(doc.markdown.contains("###### Level six heading"));
    }

    #[test]
    fn deck_and_book_are_deterministic_and_cover_features() {
        let a = generate_deck(9, 8);
        let b = generate_deck(9, 8);
        assert_eq!(a.markdown, b.markdown);
        assert!(a.markdown.contains("\n---\n"));
        assert!(a.stats.column_groups >= 1 && a.stats.png_images >= 1);

        let t1 = generate_book_tree(11, 5);
        let t2 = generate_book_tree(11, 5);
        assert_eq!(t1.files, t2.files);
        let names: Vec<&str> = t1.files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(names.contains(&"book.toml"));
        assert!(names.contains(&"src/SUMMARY.md"));
        assert!(names.contains(&"src/introduction.md"));
        assert!(names.contains(&"src/deep-dive.md"));
        assert!(names.iter().any(|p| p.ends_with("includes/snippet.md")));
        let summary = t1.files.iter().find(|(p, _)| p == "src/SUMMARY.md").unwrap();
        assert!(summary.1.contains("[Introduction](introduction.md)"));
        assert!(summary.1.contains("  - [Deep Dive](deep-dive.md)"));
    }
}
