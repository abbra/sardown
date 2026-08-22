use merman::svg::HeadlessRenderer;
use sardown_ast::BlockNode;
use std::collections::HashMap;

#[derive(Clone)]
pub struct CompiledDiagram {
    pub width: f32,
    pub height: f32,
    /// The SVG parsed into a render-ready `usvg::Tree` with the document's own font database
    /// (see [`svg_tree_options`]) -- built once here, so neither this crate nor
    /// `sardown-pdf`'s emission loop ever has to re-parse the markup a second time.
    pub tree: usvg::Tree,
}

pub type DiagramTable = HashMap<String, CompiledDiagram>;

/// usvg needs a font database to shape `<text>` elements into glyph outlines. Reuses the
/// document's own font database (already respects `typography.font_dirs`/`use_system_fonts`, and
/// was already loaded once for the rest of the document's text) instead of building a fresh,
/// system-fonts-only one from scratch -- cheaper (cloning metadata beats re-scanning disk), and
/// gives diagram text access to the same custom fonts the rest of the document uses instead of
/// silently ignoring `font_dirs` for diagrams specifically.
///
/// Real-world SVGs (e.g. Graphviz output, which commonly emits
/// `font-family="Helvetica,sans-Serif"` -- note the capital S) often name literal fonts that
/// aren't installed, and usvg's own font-family parser only recognizes the lowercase CSS generic
/// keywords, so "sans-Serif" parses as a literal name too and also fails to match. usvg's default
/// font selector unconditionally appends `fontdb::Family::Serif` as its last-resort fallback once
/// every requested family fails -- but a fontconfig-advertised generic alias (e.g. serif ->
/// "FreeSerif") can point at a font that isn't actually installed, and `fontdb::Database::query`
/// has no further fallback of its own once every requested family fails to match a loaded face.
/// So both the serif and sans-serif generic aliases are repointed at whatever's actually loaded,
/// guaranteeing usvg's last-resort fallback always resolves to a real, usable font instead of
/// silently dropping the text -- this was a real, reported bug: diagram shapes rendered fine
/// (pure geometry, no font dependency) while every text label silently vanished.
///
/// Every SVG that ends up in a `DiagramTable` -- Mermaid output *and* embedded `.svg` images --
/// is parsed through these exact options exactly once, at collection time; consumers clone the
/// resulting trees instead of re-parsing.
pub fn svg_tree_options(font_data: &fontdb::Database) -> usvg::Options<'static> {
    let mut fontdb = font_data.clone();
    ensure_resolvable_generic_families(&mut fontdb);
    usvg::Options { fontdb: std::sync::Arc::new(fontdb), ..Default::default() }
}

fn ensure_resolvable_generic_families(fontdb: &mut fontdb::Database) {
    let Some(fallback) = fontdb.faces().next().and_then(|face| face.families.first().map(|(name, _)| name.clone())) else {
        return;
    };
    for family in [fontdb::Family::Serif, fontdb::Family::SansSerif] {
        let alias = fontdb.family_name(&family).to_string();
        let alias_resolves = fontdb.faces().any(|face| face.families.iter().any(|(name, _)| *name == alias));
        if alias_resolves {
            continue;
        }
        match family {
            fontdb::Family::Serif => fontdb.set_serif_family(fallback.clone()),
            fontdb::Family::SansSerif => fontdb.set_sans_serif_family(fallback.clone()),
            _ => unreachable!("only Serif and SansSerif are iterated above"),
        }
    }
}

pub fn compile_diagrams(ast: &[BlockNode], svg_options: &usvg::Options) -> DiagramTable {
    let renderer = HeadlessRenderer::new();
    let mut table = HashMap::new();
    collect(ast, &renderer, &mut table, svg_options);
    table
}

fn collect(ast: &[BlockNode], renderer: &HeadlessRenderer, table: &mut DiagramTable, svg_options: &usvg::Options) {
    for block in ast {
        match block {
            BlockNode::MermaidDiagram { id, source, line, column, file } => {
                let fence_location = |line: usize, column: usize| match file {
                    Some(f) => format!("{}:{line}:{column}", f.display()),
                    None => format!("line {line}, column {column}"),
                };
                match renderer.render_resvg_compatible_svg_sync(source) {
                    Ok(Some(resvg_safe_svg)) => {
                        let svg = resvg_safe_svg.into_string();
                        match usvg::Tree::from_str(&svg, svg_options) {
                            Ok(tree) => {
                                let size = tree.size();
                                table.insert(id.clone(), CompiledDiagram { width: size.width(), height: size.height(), tree });
                            }
                            Err(e) => {
                                eprintln!("warning: merman produced unparseable SVG for the Mermaid diagram at {}: {e}", fence_location(*line, *column))
                            }
                        }
                    }
                    Ok(None) => eprintln!("warning: merman produced no output for the Mermaid diagram at {}", fence_location(*line, *column)),
                    Err(e) => {
                        // merman's DiagramParse errors carry a byte span *inside the diagram's
                        // own source* pointing at the actual offending token -- reporting only
                        // the opening fence's location (the fallback below) left no way to find
                        // the real problem in anything but a one-line diagram.
                        let location = match diagram_parse_span(&e) {
                            Some(span) => {
                                let (inner_line, inner_column) = line_col_at(source, span.start);
                                fence_location(line + inner_line, inner_column)
                            }
                            None => fence_location(*line, *column),
                        };
                        eprintln!("warning: failed to render the Mermaid diagram at {location}: {e}");
                    }
                }
            }
            BlockNode::Blockquote { content } => collect(content, renderer, table, svg_options),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, renderer, table, svg_options);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    collect(column, renderer, table, svg_options);
                }
            }
            _ => {}
        }
    }
}

fn diagram_parse_span(error: &merman::svg::HeadlessError) -> Option<merman::SourceSpan> {
    match error {
        merman::svg::HeadlessError::Parse(merman::Error::DiagramParse { diagnostic, .. }) => diagnostic.span(),
        _ => None,
    }
}

/// 1-indexed (line, column) for a byte offset into `text`. Column counts characters (not bytes)
/// since the start of the line. Mirrors sardown-ast's identical helper for the same purpose --
/// small and self-contained enough that a shared crate for just this isn't worth it.
fn line_col_at(text: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &text[..byte_offset.min(text.len())];
    let line = prefix.matches('\n').count() + 1;
    let column = match prefix.rfind('\n') {
        Some(i) => prefix[i + 1..].chars().count() + 1,
        None => prefix.chars().count() + 1,
    };
    (line, column)
}
