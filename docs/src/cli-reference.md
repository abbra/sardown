# Command-Line Reference

```
sardown <COMMAND>
```

Four subcommands: `render` (a single Markdown file), `render-book` (an
mdBook-style source tree), `render-slides` (a slide deck split on `---`),
and `bench` (generate seeded benchmark input and time the pipeline
rendering it).

## `render`

```
sardown render [OPTIONS] --output <OUTPUT> <INPUT>
```

| Argument/Option | Description |
|---|---|
| `<INPUT>` | Path to the Markdown file to render |
| `-o, --output <OUTPUT>` | Path to write the output PDF (required) |
| `--style <STYLE>` | Path to a stylesheet TOML file. Falls back to built-in defaults if omitted — see [Styling Your Documents](./styling/index.md) |
| `--title <TITLE>` | Document title, available to header/footer templates as `{title}`. Overrides `[document].title` from the stylesheet if both are given |
| `--author <AUTHOR>` | Document author, available to header/footer templates as `{author}`. Overrides `[document].author` from the stylesheet if both are given |
| `--date <DATE>` | Document date, available to header/footer templates as `{date}`. Overrides `[document].date` from the stylesheet if both are given; defaults to today's date if neither is set |
| `-h, --help` | Print help |

Example:

```bash
sardown render report.md -o report.pdf --style docs/style-examples/eu-a4.toml
```

## `render-book`

```
sardown render-book [OPTIONS] --output <OUTPUT> <BOOK_ROOT>
```

| Argument/Option | Description |
|---|---|
| `<BOOK_ROOT>` | Path to the book's root directory (containing `book.toml` and/or `src/SUMMARY.md`) |
| `-o, --output <OUTPUT>` | Path to write the output PDF (required) |
| `--style <STYLE>` | Path to a stylesheet TOML file. Falls back to `<book_root>/style.toml` if present, then to built-in defaults — see [Rendering Books](./books/index.md) |
| `--title <TITLE>` | Document title, available to header/footer templates as `{title}`. Overrides `[document].title` from the stylesheet if both are given |
| `--author <AUTHOR>` | Document author, available to header/footer templates as `{author}`. Overrides `[document].author` from the stylesheet if both are given |
| `--date <DATE>` | Document date, available to header/footer templates as `{date}`. Overrides `[document].date` from the stylesheet if both are given; defaults to today's date if neither is set |
| `-h, --help` | Print help |

Example:

```bash
sardown render-book my-book -o my-book.pdf
```

## `render-slides`

```
sardown render-slides [OPTIONS] --output <OUTPUT> <INPUT>
```

| Argument/Option | Description |
|---|---|
| `<INPUT>` | Path to the Markdown source file, split into slides on `---` -- see [Slide Decks](./slides.md) |
| `-o, --output <OUTPUT>` | Path to write the output PDF (required) |
| `--style <STYLE>` | Path to a stylesheet TOML file. Falls back to built-in defaults if omitted |
| `--title <TITLE>` | Document title, available to header/footer templates as `{title}`. Overrides `[document].title` from the stylesheet if both are given |
| `--author <AUTHOR>` | Document author, available to header/footer templates as `{author}`. Overrides `[document].author` from the stylesheet if both are given |
| `--date <DATE>` | Document date, available to header/footer templates as `{date}`. Overrides `[document].date` from the stylesheet if both are given; defaults to today's date if neither is set |
| `-h, --help` | Print help |

Example:

```bash
sardown render-slides deck.md -o deck.pdf --style slides-style.toml
```


## `bench`

```
sardown bench [OPTIONS]
```

Generates deterministic, feature-complete Markdown input from a seed,
renders it through the full production pipeline, and prints a per-stage
timing table (min/mean/max across iterations). Useful for comparing builds
and for producing reproducible sample documents — the same seed always
regenerates byte-identical input.

| Argument/Option | Description |
|---|---|
| `--seed <SEED>` | PRNG seed (default `42`). Same seed ⇒ byte-identical generated input |
| `--mode <MODE>` | What to generate and render: `render` (single document, default), `book` (mdBook tree with SUMMARY.md, includes, cross-file links), or `slides` (`---`-split deck) |
| — | In `slides` mode without `--style`, a 16:9 landscape page (338.667 × 190.5 mm) is applied automatically so the benchmark renders as real slides rather than portrait book pages; passing `--style` keeps full control |
| `--pages <PAGES>` | Output volume: approximate page count for `render`/`book`, slide count for `slides` (default `25`) |
| `--iterations <N>` | Full-pipeline repetitions in the timing table (default `3`) |
| `--style <STYLE>` | Stylesheet TOML passed through to the pipeline |
| `--markdown-out <PATH>` | Write the generated Markdown here (`render`/`slides` modes) |
| `--book-dir <DIR>` | Directory for the generated book tree (`book` mode; defaults to a fresh temp directory) |
| `-o, --output <OUTPUT>` | Where to write the rendered PDF; omitted means bytes are discarded after timing |
| `-h, --help` | Print help |

The run prints the seed, a coverage summary counting every supported
construct the generated input contains (headings, lists, tables, code
blocks, images by type, diagrams, column groups, links), then the timing
table.

Example:

```bash
sardown bench --seed 42 --pages 30 --iterations 5 -o bench.pdf
```

## Output and diagnostics

Both commands print stage-by-stage progress (parsing, highlighting,
compiling diagrams, laying out pages, rendering the PDF, writing output)
with per-stage timing to stderr, plus warnings for anything skipped or
degraded during the render (an unresolvable font, a failed diagram, an
unreachable link, an external image that wasn't fetched) — see
[Troubleshooting](./troubleshooting.md) for what each warning means. A
render still succeeds and produces output even when it prints warnings;
the affected content is simply left out or falls back to a default rather
than aborting the whole document.

## Exit status

Both commands exit non-zero and print an error (not just a warning) when
the render can't proceed at all — a missing/unreadable input file, a
missing `SUMMARY.md`, or an invalid stylesheet (bad TOML syntax, an
unknown placeholder in a header/footer template, or `[page]` setting only
one of `width_mm`/`height_mm`).
