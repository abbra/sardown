use md2pdf_ast::{BlockNode, HighlightedToken};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        Self::with_style(&md2pdf_style::Stylesheet::default())
    }

    /// Falls back to `InspiredGitHub` (with a warning) for a syntax theme name syntect doesn't
    /// bundle, rather than panicking -- matching this project's established convention of
    /// degrading gracefully instead of aborting a whole render over one bad config value.
    pub fn with_style(style: &md2pdf_style::Stylesheet) -> Self {
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
        for line in source.split_inclusive('\n') {
            let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) else {
                tokens.push(HighlightedToken { text: line.to_string(), color: [0, 0, 0] });
                continue;
            };
            for (style, text) in ranges {
                tokens.push(HighlightedToken { text: text.to_string(), color: color_of(style) });
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
