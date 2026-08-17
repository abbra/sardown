# Command-Line Reference

```
md2pdf <COMMAND>
```

Two subcommands: `render` (a single Markdown file) and `render-book` (an
mdBook-style source tree).

## `render`

```
md2pdf render [OPTIONS] --output <OUTPUT> <INPUT>
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
md2pdf render report.md -o report.pdf --style docs/style-examples/eu-a4.toml
```

## `render-book`

```
md2pdf render-book [OPTIONS] --output <OUTPUT> <BOOK_ROOT>
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
md2pdf render-book my-book -o my-book.pdf
```

## `render-slides`

```
md2pdf render-slides [OPTIONS] --output <OUTPUT> <INPUT>
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
md2pdf render-slides deck.md -o deck.pdf --style slides-style.toml
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
