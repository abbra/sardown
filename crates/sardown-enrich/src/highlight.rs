use sardown_ast::{BlockNode, HighlightedToken};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Whether `ast` contains at least one fenced code block.
///
/// `Highlighter::with_style` builds a full syntect `Highlighter` (loading every default syntax
/// definition and the complete theme) before deciding there is nothing to do -- that load takes
/// well over a second and is pure overhead for the common case of a document with no code
/// blocks. Callers use this predicate to skip the construction entirely.
pub fn ast_contains_code_block(ast: &[BlockNode]) -> bool {
    ast.iter().any(|block| matches!(block, BlockNode::CodeBlock { .. }))
}

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        Self::with_style(&sardown_style::Stylesheet::default())
    }

    /// Falls back to `InspiredGitHub` (with a warning) for a syntax theme name syntect doesn't
    /// bundle, rather than panicking -- matching this project's established convention of
    /// degrading gracefully instead of aborting a whole render over one bad config value.
    pub fn with_style(style: &sardown_style::Stylesheet) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme_name = &style.code_block.syntax_theme;
        let theme = theme_set.themes.get(theme_name).cloned().unwrap_or_else(|| {
            eprintln!("warning: unknown syntax theme {theme_name:?}; falling back to InspiredGitHub");
            theme_set.themes["InspiredGitHub"].clone()
        });
        Self { syntax_set, theme }
    }

    pub fn highlight(&self, ast: Vec<BlockNode>) -> Vec<BlockNode> {
        ast.into_iter().map(|block| self.highlight_block(block)).collect()
    }

    fn highlight_block(&self, block: BlockNode) -> BlockNode {
        match block {
            BlockNode::CodeBlock { language, tokens } => {
                let raw: String = tokens.into_iter().map(|t| t.text).collect();
                let highlighted = self.highlight_source(&raw, language.as_deref());
                BlockNode::CodeBlock { language, tokens: highlighted }
            }
            BlockNode::Blockquote { content } => BlockNode::Blockquote { content: self.highlight(content) },
            BlockNode::List { ordered, start, items } => {
                let items = items.into_iter().map(|item| self.highlight(item)).collect();
                BlockNode::List { ordered, start, items }
            }
            BlockNode::Columns(columns) => BlockNode::Columns(columns.into_iter().map(|column| self.highlight(column)).collect()),
            other => other,
        }
    }

    fn syntax_for(&self, language: Option<&str>) -> &SyntaxReference {
        language.and_then(|lang| self.syntax_set.find_syntax_by_token(lang)).unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    fn highlight_source(&self, source: &str, language: Option<&str>) -> Vec<HighlightedToken> {
        let syntax = self.syntax_for(language);
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut tokens = Vec::new();
        // Logged once per code block, not once per failing line: a highlighting engine error is
        // typically a per-block condition (e.g. a syntax definition it can't process), and this
        // avoids a wall of identical warnings for one broken code block.
        let mut warned = false;
        for line in source.split_inclusive('\n') {
            match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        tokens.push(HighlightedToken { text: text.to_string(), color: color_of(style) });
                    }
                }
                Err(e) => {
                    if !warned {
                        eprintln!("warning: syntax highlighting failed ({e}); rendering the rest of this code block unhighlighted");
                        warned = true;
                    }
                    tokens.push(HighlightedToken { text: line.to_string(), color: [0, 0, 0] });
                }
            }
        }
        tokens
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn color_of(style: Style) -> [u8; 3] {
    [style.foreground.r, style.foreground.g, style.foreground.b]
}
