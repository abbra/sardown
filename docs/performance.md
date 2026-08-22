# sardown performance architecture & limitations

This document describes *where the time goes* in the Markdown→PDF pipeline and, more
importantly, **which limitations are hard ceilings vs. which were removable**, based on a full
optimization pass across all eight crates (release-build measurements). It is meant to save the
next person from re-deriving what we already know — including several dead ends that look
obvious but do not actually help.

All numbers below are `cargo build --release` wall-clock of the **"Laying out pages"** stage on
this machine, via `timed_stage` in `sardown-cli/src/main.rs`. Two reference inputs:

| Input | Why it matters |
|---|---|
| `~/workspace/synta/docs` — 816-page mixed prose+code book (`eu-a4`) | the realistic corpus; dominated by cosmic-text internals |
| `/tmp/bench-code.md` — one ~3,200-line code block | maximally biased toward per-glyph / per-node shaping cost |

---

## 1. Pipeline and where cost concentrates

The render pipeline (`sardown-cli/src/main.rs`) is a strict sequence:

```
parse (sardown-ast) → enrich: syntect highlight + mermaid (sardown-enrich)
                    → layout/paginate (sardown-layout)   ← ~90% of total time here
                    → PDF emission (sardown-pdf via krilla)
```

Within the **layout** stage, nearly all cost is in *shaping* — turning text into positioned
glyph runs using `cosmic-text` (`sardown-layout/src/shape.rs`, `paginate.rs`). Everything else
(pagination decisions, block layout math, TOC, numbering) is comparatively cheap. So the
performance story is essentially "how many times do we make cosmic-text shape the same bytes?"

Layout entry points (real symbols):

- `layout_with_header_footer` — top-level per-document (`sardown-layout/src/header_footer.rs`, re-exported from `lib.rs`)
- `prepare_layout_assets` / `layout_with_assets` — scale-independent assets hoisted once (#4)
- `shape_paragraph`, `shape_rich_paragraph`, `place_inline_content`, `place_shaped_runs` — the shaping hot path (`shape.rs`, `paginate.rs`)

---

## 2. The hard ceiling: cosmic-text 0.19's per-glyph font resolution

**This is the single most important limitation, and it cannot be removed from sardown.**

Profiling (flat profiles + ablation; dwarf callgraphs don't decode on this host) shows that
roughly **~60% of the remaining layout time** lives inside `cosmic-text 0.19`'s internal shaping:

- `FontFallbackIter::next` — walking fallback fonts per glyph
- `get_font_supported_codepoints_in_word` + associated `BTreeSet` ops — checking which faces
  cover each codepoint, done **per word/glyph**

These run inside cosmic-text during every `Buffer::shape_line`; sardown has no API hook to skip
or pre-warm them. Consequences:

1. It is the ceiling on realistic corpora. On the 816-page book the layout stage sits at
   **~6.0–6.1s** and stays there even when every *removable* sardown-side cost is gone — because
   most of that time is this internal work, not anything we call directly (see §5).
2. It explains why some "obvious" micro-opts are near-neutral on mixed content but large on
   code-heavy inputs (§5).

**The only real levers past here:** upgrade `cosmic-text` to a version with cheaper fallback /
codepoint support, or parallelize shaping across independent regions (§7) — the latter is also
blocked by cosmic-text/fontdb thread-safety (see below), so it's really "wait for upstream" today.

---

## 3. Limitations we found and what was done about them

Each entry: the limit, evidence, status (**Fixed** / **Open-ceiling**), and the commit(s).
`#n` refers to the original numbered issue list; `H`/`I`/`J` are the code-block sub-work items.

| # | Limitation found | Evidence | Status → fix | Commit(s) |
|---|---|---|---|---|
| 1 | Hyphenation shaped a word once **per candidate split** (N shapes per hyphenatable word) | `hyphenate.rs` re-shaped the same word for each break point | **Fixed** → shape once (`ShapedWord`), derive prefix widths via `partition_point` | `34a15df` |
| 2 | Table cells measured longest-line and longest-word in *separate* passes; column-width pass then shaped again at placement time | `table.rs::measure_cell`, `paginate.rs` re-shaped each cell twice | **Fixed** → one shaping pass returns both (`CellMeasure`) + reuse via `shape_row_cells`/`place_shaped_runs` | `84d0374` (+ see #3) |
| 3 | Inline content shaped in a measure pass, then shaped *again* identically at placement time (every paragraph/table cell paid shaping twice) | `paginate.rs::place_inline_content` called shape-then-place with the same width | **Fixed** → factor out `place_shaped_runs`; callers that already have runs place them directly | `5775f94` |
| 4 | Slide auto-shrink rebuilt all scale-independent assets *and* re-decoded pixel buffers on every shrink iteration | `slides/lib.rs` + repeated image decode in the loop | **Fixed** → `prepare_layout_assets`/`layout_with_assets`; `DecodedImage.rgba8: Arc<Vec<u8>>` shared across iterations | `8d1937d` (+ #13) |
| 5 | Per-glyph source-span lookup was a linear scan: `spans.iter().position(...)` per glyph → O(glyphs×spans) in rich/code lines | `shape.rs::span_index_for`; measured ~47% layout cut on code-heavy input when removed | **Fixed** → single forward-only cursor (amortized O(1)), spans tile contiguously & offsets are monotonic; guard resets if an offset ever regressed | `5eaa8d8` |
| 6 | Literal font-family name checked by scanning *every* loaded face's alias list (`db.faces().any(...)`, case-insensitive) — called **per inline node**, so one code block re-scanned the whole DB for the same few names | `shape.rs::resolve_family`; generic keywords already short-circuit before any scan, only named families paid | **Fixed** → thread-local cache keyed by `(database pointer, name)` (`FAMILY_KNOWN_CACHE`); each name scans at most once; distinct font systems never share entries (mirrors the existing monospace probe cache) | `a6b2cfa` |
| 7 | The syntect highlighter (theme + grammar loads — heavy) was built even when a document has **no fenced code blocks** | `sardown-enrich/highlight.rs`; pure overhead for prose-only docs | **Fixed** → gate on `ast_contains_code_block()` before constructing the highlighter (CLI + slides both) | `347ae46` |
| 8 | Repeated images re-decoded/re-attached per emission; each copy re-built a full raster `Image` at PDF write time | `sardown-pdf/src/lib.rs::render_pdf` loop over pages | **Fixed** → build `raster_cache: HashMap<&str, Image>` once before the page loop | `ee2eae9` |
| 10 | Mermaid/SVG diagrams re-parsed (`usvg`) on every repeated reference | same loop as #8 | **Fixed** → `svg_cache: HashMap<&str, usvg::Tree>` keyed by diagram_id, built once | `ee2eae9` |
| 9 | Repeating header/footer zones were re-shaped *per page* — huge for hundreds of pages | `header_footer.rs`; a book's running head shaped ~800 times | **Fixed** → `ShapedZoneCache` keyed by `(resolved text, font size bits, family, color)` shared across all pages | `6915770` |
| 13 | Per-image base-dir canonicalization + data-URI handling re-resolved inside the decode loop; a non-canonicalizable base made embedded images undecodable | `image.rs::decode_images`/`resolve_within_base` | **Fixed** → canonicalize once (`canonical_base`); always process data URIs regardless of base resolution | `9b1502f` |
| J | `shrink_to_fit` code blocks shaped their widest line *twice* (a measurement pass ≈ 4.5s on the book) purely to compute a width that, for monospace faces, is trivially computable | `paginate.rs::CodeBlock` arm used `measure_widest_line_pt`; measured ~4.5s of redundant shaping | **Fixed** → `estimate_code_natural_width_pt` / `monospace_advance_pt`: if the resolved face is provably monospaced, width = `char_count × advance` with no shaping; non-monospace or any line containing a `\t` returns `None` and falls back to exact measurement | (see commit below) |
| H | Code rendered *with* ligatures: `fi`/`ff`/`fl` collapse to single-cell glyphs, shifting every column after them left — broke code alignment **and** made any width estimate overstate | 80-char line with 3×`fi`: ligatures on → 77 glyphs / 462pt; off → 80 glyphs / 480.00pt (Adwaita Mono @10) | **Fixed** → `ligatures: bool` on `InlinePlacement`; code shapes with `liga/dlig/clig` disabled (`no_ligature_features()`), prose keeps them — this also makes the J estimate exact | `72a093a` |

> The estimator (J) and ligature fix (H) are deliberately **separate commits** from #5/#6 above
> because they are coupled: the monospace width is only *exactly* `char_count × advance` once
> code is shaped without ligatures. That dependency is why H exists at all, and why J trusts it.

---

## 4. Structural limits that were evaluated but deliberately left

These looked like candidates; we measured or reasoned them out as not worth the cost/risk:

- **#11 — per-code-block style clone.** Negligible (tiny allocation), skipped.
- **#12 — `line_text` cloning in `shape_rich_paragraph`.** Each flushed run clones its line's text string so krilla can build a ToUnicode CMap for it. Avoiding the clone means changing how glyph clusters correlate against that string (`cluster: Range<usize>` is correlated by index into `line_text`). Invasive, and the cost is small relative to shaping; deferred.
- **#14 — rayon parallelism (see §7).** The "obvious" big win, but it needs a new dependency **and** fights cosmic-text/fontdb thread-safety plus krilla's non-thread-safe emission. Deferred pending upstream changes.

---

## 5. Measured results (release build)

### Realistic corpus — `synta` 816-page book (`eu-a4`)
| Stage | Baseline | After all optimizations (current HEAD, incl. #5+#6) | Change |
|---|---|---|---|
| Laying out pages | **11.10s** | **~6.0–6.1s** (A/B: 6.1/6.2/6.0 before vs 5.9/5.8/6.3 after #5+#6 — interleave, so ~flat at the ceiling) | **−46%** end-to-end; the last two opts are within noise here |

### Code-heavy input — `/tmp/bench-code.md` (one giant block)
| Stage | Baseline | After all optimizations | Change |
|---|---|---|---|
| Laying out pages | 1.48s | **~0.8s** (#5+#6 A/B: ~1.6→~0.8, i.e. these two alone ≈ **−47%** here) | −46% |

### Table-heavy — `bench-tables.md` (60×20 tables)
| Stage | Baseline | After all optimizations | Change |
|---|---|---|---|
| Laying out pages | 0.19s | ~0.10s (#2 single-pass cell measurement) | −47% |

### The key nuance (do not over-claim #5/#6 on realistic content)
On the **mixed** book, adding #5+#6 is statistically flat (~0.1s, ranges interleave): that
corpus's cost is dominated by cosmic-text internals (§2), and most code blocks use `monospace`
which short-circuits before the #6 family-name path anyway. On a **code-only** input they are
large (~47%). So these two are worth keeping (correct, rendering-identical, no warnings) because
they protect worst-case code-heavy documents — but do not expect them to move an aggregate book
number.

### Correctness invariants preserved throughout
- Rendered PDFs **byte-identical** for the slide-deck fixtures; golden/visual-regression images pass (3/3).
- No build warnings anywhere in the workspace after all changes.
- Only failing tests are pre-existing/environmental: `pdfa_conformance` (needs external `verapdf`) and two `sardown-layout/tests/table.rs` width assertions that fail **identically at clean HEAD** on this machine's fonts (<0.5pt / 45pt thresholds vs the installed sans-serif metrics) — not regressions from any of these changes.

---

## 6. Dead ends (things that *worsened* or did nothing)

- **Naming an explicit monospace font instead of `monospace`.** Tried resolving code to a
  named "Adwaita Mono" in the hope it would skip cosmic-text's generic matching: measured
  **slower** (~11.32s vs ~7s baseline). The per-word monospace *codepoint-support* path runs for
  any monospace face; naming doesn't remove it. Don't do this.
- **Expecting sardown-side micro-opts to cut the book below ~6s.** They cannot — see §2/§5. The
  removable work (redundant shaping passes, repeated decode/shape of repeating content) is done;
  what's left is inside cosmic-text.

---

## 7. Where real remaining headroom actually lives

Ranked by expected impact on the realistic book, *today*:

1. **`cosmic-text` upgrade** (cheaper `FontFallbackIter` / codepoint support). The only way to
   touch the ~60% internal ceiling. Blocked purely on upstream availability; no sardown API can reach it.
2. **Parallelize independent regions (`#14`, rayon).** Pages and top-level blocks are laid out
   sequentially and mostly independently. Big in theory, but: `FontSystem`/fontdb is not safe to
   shape from multiple threads with one instance (per the existing `Pdfium::new` OnceLock precedent
   in `sardown-cli/tests/visual_regression.rs`, C libs here are single-instance), so you'd need a
   font-system **pool** or thread-confined systems — meaningful engineering, and it only helps once
   the §1 ceiling is also cheaper. Defer until upstream improves fallback cost.
3. **Cross-region glyph caching beyond zones.** We cache shaped *header/footer* zones (#9) because
   they repeat verbatim; body text generally doesn't. Only worth pursuing if real corpora show
   heavy repetition (e.g., repeated tables/figures); not a general win.

---

## 8. Second optimization pass (independent review, 2026-08)

A fresh review re-verified §2–§6 at HEAD (profile still ~63% cosmic-text internals; no
sardown-side symbol above ~1.5%) and found five remaining removable costs. All numbers below are
`cargo build --release` "Laying out pages" wall-clock on the same machine, same corpora as §5.

| # | What was found | Fix | Measured effect |
|---|---|---|---|
| 15 | **Hyphenation shaped every word of every paragraph with no cache.** The greedy-wrap simulation in `hyphenate.rs` runs `shape_word` per word occurrence (a full cosmic-text `Buffer` shape each), and `" "`/`"-"` were re-shaped per inline node — while the same crate already cached the monospace probe and family-name lookups. | Thread-local `WORD_SHAPING_CACHE` keyed by (family, size bits, bold, italic, word) — the only style fields that affect advances. Inner map keyed by `str` so hits allocate nothing; capped per style. Space/hyphen widths ride the same cache. | Prose corpus with `hyphenation = true`: **1.30s → 0.48s** (was +183% over hyphenation-off, now +4%). Book with hyphenation on: **6.69s → 5.78s** (+19% → +2.5%). Hyphenation-off numbers unchanged. |
| 16 | **Slide auto-shrink deep-cloned the slide AST and the full `Stylesheet` on every shrink iteration** — including iteration 1 at scale 1.0, where `rescale_slide_content` provably writes back the values already present (each target is its own parse-time source value ×1.0). | Stylesheet cloned + overridden once per slide; each retry applies only the scale step in place (`apply_slide_scale`, which sets fields from source × scale so retries can't compound). AST clone+rescale skipped at scale 1.0 when no color/body-size overrides are set. | Removes O(iterations × slide bytes) churn per slide; layout call itself unchanged. |
| 17 | **Every diagram's SVG was parsed twice per document** — once in `sardown-enrich` (mermaid output, `Options::default()`, tree discarded after reading size) and again in `sardown-pdf`'s `svg_cache` (document-fontdb options). Embedded `.svg` files/data URIs had the same split. | `CompiledDiagram` now carries a render-ready `usvg::Tree` built once through `sardown_enrich::svg_tree_options(fontdb)` (the options builder relocated here from sardown-pdf, so text shaping uses the document's own fonts exactly as before). CLI/slides build the font system *before* enrichment; `prepare_layout_assets` threads the same options into embedded-SVG collection. PDF emission clones trees; no SVG markup is parsed at render time. | Book diagram stage 0.09s→0.15s on this corpus (one fontdb clone + tree build up front, amortized); eliminates the per-diagram re-parse entirely and matters proportionally more the more/larger a document's diagrams are. Golden-image visual regression passes byte-identical. |
| 18 | **A second full pulldown-cmark pass ran on every document** to locate ```mermaid fences, even with zero diagrams. | Gated on `markdown.contains("mermaid")` — a fence implies the substring; the same pattern as #7's highlighter gate. | Skips one full tokenization pass for diagram-free documents (parse stage is tens of ms on the book; pure win). |
| 19 | **`TextStyle.font_family` was a plain `String`, copied per inline run** at parse time, per code-block token, per list marker, and per measured hyphenation word — all for one of a handful of names. | `Arc<str>`: construction sites convert once (per document/heading/code block); every downstream clone is a refcount bump. | Part of the ~3.7% allocator/memmove profile share; not individually visible at the stage-timing level, which is exactly why it's a type fix and not a hotspot patch. |
| 20 | **PDF emission converted every raster image eagerly and allocated a fresh `Vec` per text run.** The raster cache cloned every image's full RGBA buffer up front — including images whose placements were dropped by pagination/slides shrink before emission saw them — and each `TextRun` built its own `Vec<KrillaGlyph>`. | Raster conversion is lazy (first placement, still exactly once per referenced image; missing ids draw nothing as before), and one scratch glyph buffer is reused across every run. | Removes O(unreferenced-image bytes) copies and one alloc per emitted run; invisible at stage-timing granularity, which is why both are emission-loop hygiene rather than headline wins. |

### A ceiling this pass documented rather than fixed

`svg_cache` (fix #10) prevents re-*parsing*, but a `VectorGraphic` referenced from N pages is
still *re-rendered* N times: krilla-svg walks the whole `usvg::Tree` into content ops per
placement, and (verified in krilla-svg 0.8.1 source) clones its `fontdb` per placement before
`Arc::make_mut`. No form-XObject reuse is exposed via `draw_svg`, so this is an upstream ask,
not a sardown-side fix. Don't read #10/#17 as "repeated diagrams are free" — they are merely
parse-free.

 ## Appendix A — module map (where each concern lives)

```
crates/sardown-layout/src/
  shape.rs        shaping core: resolve_family (#6), no_ligature_features/#H,
                  shape_paragraph / shape_rich_paragraph (+ span_cursor #5, place_shaped_runs),
                  measure_widest_line_pt / monospace_advance_pt / estimate_code_natural_width_pt (J)
  paginate.rs     block/pagination: layout_with_* entry, CodeBlock arm (uses J+H),
                  table path (#3 shape_row_cells reuse), LayoutAssets hoisting (#4)
  header_footer.rs  ShapedZoneCache (#9), layout_with_header_footer
  image.rs        decode_images + DecodedImage{rgba8: Arc<Vec<u8>>}, canonical_base, data URIs (#13/#4);
                  collect_svg_diagrams(ast, base_dir, &svg_options) → tree-bearing CompiledDiagram (#17)
  table.rs        column_widths / measure_cell single pass (#2)
  hyphenate.rs    ShapedWord single-pass prefix widths (#1); WORD_SHAPING_CACHE per
                  (family, size, bold, italic, word) (#15)

crates/sardown-pdf/src/lib.rs   render_pdf: raster_cache + svg_cache built once per document (#8/#10);
                                svg_cache now clones pre-built trees, no parsing at emission (#17)

crates/sardown-enrich/          lazy syntect gate (ast_contains_code_block → build highlighter, #7);
                                svg_tree_options(db) + compile_diagrams → tree-bearing CompiledDiagram (#17)
crates/sardown-slides/          auto-shrink hoisting via layout_with_assets; DecodedImage Arc reuse (#4);
                                per-slide stylesheet + scale-1.0 no-clone fast path (#16)
```

## Appendix B — how to reproduce the numbers

```bash
export PDFIUM_DYNAMIC_LIB_PATH="$PWD/.pdfium/lib"
cargo build --release -p sardown-cli
# realistic book:
./target/release/sardown render-book ~/workspace/synta/docs -o /tmp/o.pdf \
  --style docs/style-examples/eu-a4.toml        # grep "Laying out pages... done (" from stderr
# code-heavy input (worst-case for the shaping micro-opts):
./target/release/sardown render /tmp/bench-code.md -o /tmp/o.pdf \
  --style docs/style-examples/eu-a4.toml

# A/B against pre-#5/#6 state: `git show 590342a:crates/sardown-layout/src/shape.rs` into the file, rebuild.
```
