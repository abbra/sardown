# Installation

sardown is a Cargo workspace; there's no published crate or prebuilt binary
yet, so it's built from source.

## Prerequisites

- A recent stable Rust toolchain (install via [rustup](https://rustup.rs)
  if you don't already have one).
- No system libraries beyond a normal Rust toolchain are required for
  rendering itself. `image` (for embedded PNG/JPEG) and font-loading
  (`fontdb`) are pure-Rust and vendored as regular crate dependencies.

## Building

```bash
git clone <repository-url>
cd sardown
cargo build --release
```

The `sardown` binary is produced at `target/release/sardown`. Run it directly,
or install it onto your `PATH`:

```bash
cargo install --path crates/sardown-cli
```

## Verifying the build

```bash
sardown --help
```

should print the three subcommands, `render`, `render-book`, and
`render-slides` (see the
[Command-Line Reference](./cli-reference.md)).

## Fonts

By default, sardown loads whatever fonts are already installed on your
system (`use_system_fonts = true` in the default stylesheet) and shapes
text with a generic sans-serif family. If you want to use a specific named
font (e.g. `"Times New Roman"`), it needs to actually be installed as a
system font, or loaded from a directory you point the stylesheet at via
`typography.font_dirs` — see [Typography](./styling/typography.md) and
[Font Resolution](./troubleshooting.md#unknown-font-family-warnings).
