# Quick Start

## Rendering a single file

Write a Markdown file:

```markdown
# Hello, sardown

This is a paragraph with **bold**, *italic*, and a [link](https://example.com).

- One
- Two
- Three

| Feature | Supported |
|---|---|
| Tables | Yes |
| Code blocks | Yes |
```

Render it:

```bash
sardown render hello.md -o hello.pdf
```

That's it — `hello.pdf` is a fully paginated PDF/A-2b document using
sardown's built-in default styling (US Letter, sans-serif body text, black
headings).

## Rendering a book

If you have an existing mdBook-style source tree (a directory with
`book.toml` and/or `src/SUMMARY.md`), render the whole thing as one
combined PDF:

```bash
sardown render-book path/to/my-book -o my-book.pdf
```

Chapters are combined in `SUMMARY.md` order, each starting on its own page,
with cross-chapter links resolved to working internal PDF links. See
[Rendering Books](./books/index.md) for the full details.

## Applying a stylesheet

Both commands accept `--style <path>` to point at a stylesheet TOML file
that overrides sardown's defaults:

```bash
sardown render hello.md -o hello.pdf --style my-style.toml
```

```toml
# my-style.toml
[page]
format = "a4"
margin_mm = 20.0

[typography]
font_family = "Helvetica"
body_size_pt = 11.0
```

`render-book` additionally auto-discovers a `style.toml` file at the book's
root if `--style` isn't given explicitly. See
[Styling Your Documents](./styling/index.md) for everything a stylesheet
can control, or jump straight to the
[Style Presets Gallery](./styling/presets.md) for ready-made examples
(US/EU regional conventions, an academic paper preset, technical manual and
tutorial-guide presets, and a fiction/novel preset).
