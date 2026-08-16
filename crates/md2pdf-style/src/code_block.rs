use crate::Color;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelStyle {
    #[default]
    Corner,
    HeaderBar,
    Inline,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct CodeBlockDefaultStyle {
    pub background: Color,
    pub font_family: String,
    pub font_size_pt: f32,
    pub label_color: Color,
    pub label_background: Color,
}

impl Default for CodeBlockDefaultStyle {
    fn default() -> Self {
        CodeBlockDefaultStyle {
            background: Color([245, 245, 245]),
            font_family: "monospace".to_string(),
            font_size_pt: 10.0,
            label_color: Color([102, 102, 102]),
            label_background: Color([224, 224, 224]),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CodeLanguageStyle {
    pub label: Option<String>,
    pub background: Option<Color>,
    pub font_family: Option<String>,
    pub font_size_pt: Option<f32>,
    pub label_color: Option<Color>,
    pub label_background: Option<Color>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct CodeBlockStyle {
    pub syntax_theme: String,
    pub label_style: LabelStyle,
    pub default_label: String,
    pub default: CodeBlockDefaultStyle,
    pub languages: BTreeMap<String, CodeLanguageStyle>,
}

impl Default for CodeBlockStyle {
    fn default() -> Self {
        CodeBlockStyle {
            syntax_theme: "InspiredGitHub".to_string(),
            label_style: LabelStyle::Corner,
            default_label: "text".to_string(),
            default: CodeBlockDefaultStyle::default(),
            languages: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCodeBlockStyle {
    pub background: Color,
    pub font_family: String,
    pub font_size_pt: f32,
    pub label_color: Color,
    pub label_background: Color,
    pub label: String,
}

impl CodeBlockStyle {
    /// `language` is the fence's own token (e.g. `"rust"` for ` ```rust `), or `None` for an
    /// untagged fence. See the design spec's §5 for the exact label precedence this implements.
    pub fn resolve(&self, language: Option<&str>) -> ResolvedCodeBlockStyle {
        let lang_override = language.and_then(|l| self.languages.get(l));
        let label = match (language, lang_override.and_then(|o| o.label.clone())) {
            (_, Some(explicit)) => explicit,
            (Some(lang), None) => title_case(lang),
            (None, None) => self.default_label.clone(),
        };
        ResolvedCodeBlockStyle {
            background: lang_override.and_then(|o| o.background).unwrap_or(self.default.background),
            font_family: lang_override.and_then(|o| o.font_family.clone()).unwrap_or_else(|| self.default.font_family.clone()),
            font_size_pt: lang_override.and_then(|o| o.font_size_pt).unwrap_or(self.default.font_size_pt),
            label_color: lang_override.and_then(|o| o.label_color).unwrap_or(self.default.label_color),
            label_background: lang_override.and_then(|o| o.label_background).unwrap_or(self.default.label_background),
            label,
        }
    }
}

fn title_case(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
