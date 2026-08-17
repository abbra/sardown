# Slide Decks

`md2pdf render-slides` renders a Markdown file as a slide deck: one page per
slide, laid out on a fixed page size rather than flowing continuously like
`render`/`render-book` do.

This is **not** a Marp-compatible renderer. There is no HTML/CSS
interpretation, no theme cascade, and no variable fonts -- a deck is
written using md2pdf's own conventions below.

## Splitting a deck into slides

A `---` line (a thematic break) starts a new slide, exactly like it does in
`render`/`render-book` -- no new Markdown syntax is introduced. A deck with
no `---` at all is a single slide.

## Choosing a slide's layout

Put a line reading `@layout: <name>` as the very first thing in a slide
(before any heading or other content) to select a named layout for that
slide:

```markdown
@layout: title

# My Presentation
```

A slide with no `@layout:` line uses `[slides].default_layout`. Every
layout name used anywhere in the deck -- as a directive or as
`default_layout` -- must have a matching `[slides.layouts.<name>]` table in
the stylesheet, or the render fails with an error naming the missing
layout. If the stylesheet has no `[slides]` section at all, every slide
uses one plain built-in layout (top-anchored, left-aligned, no
background).

## Columns

Use a Markdown table for side-by-side content -- there is no separate
"columns" syntax.

## Auto-shrink-to-fit

If a slide's content doesn't fit the configured page size at its layout's
normal font sizes, md2pdf automatically retries at smaller font sizes
(stepping down in 5% increments) until it fits one page, down to
`[slides].min_scale` (default `0.5`, i.e. never below half size). If even
the smallest allowed size still overflows, md2pdf renders all the overflow
content anyway (spanning more than one physical page for that slide) and
prints a warning -- content is never silently dropped.

## Non-goals

Table of contents generation, background images (only solid background
colors are supported), and animations/transitions/speaker notes are not
supported.

See the [Stylesheet Reference](./stylesheet-reference.md#slides) for every
`[slides]`/`[slides.layouts.<name>]` field.
