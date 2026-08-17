use crate::Color;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct HeaderZones {
    pub left: String,
    pub center: String,
    pub right: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderFooterMode {
    #[default]
    Uniform,
    TwoSided,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct HeaderFooterStyle {
    pub enabled: bool,
    pub font_family: String,
    pub font_size_pt: f32,
    pub color: Color,
    pub mode: HeaderFooterMode,
    pub suppress_on_chapter_start: bool,
    pub uniform: HeaderZones,
    pub odd: HeaderZones,
    pub even: HeaderZones,
}

impl Default for HeaderFooterStyle {
    fn default() -> Self {
        HeaderFooterStyle {
            enabled: false,
            font_family: "sans-serif".to_string(),
            font_size_pt: 9.0,
            color: Color([102, 102, 102]),
            mode: HeaderFooterMode::Uniform,
            suppress_on_chapter_start: true,
            uniform: HeaderZones::default(),
            odd: HeaderZones::default(),
            even: HeaderZones::default(),
        }
    }
}

const VALID_PLACEHOLDERS: [&str; 6] = ["h1", "h2", "page", "total_pages", "title", "author"];

/// Checks every `{...}` token in `template` against `VALID_PLACEHOLDERS`, so a typo'd placeholder
/// name is a load-time error naming the bad token rather than silently rendering as literal text
/// (or an unterminated `{` silently swallowing the rest of the template) at render time.
fn validate_template(template: &str, field_name: &str) -> anyhow::Result<()> {
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after_open = &rest[start + 1..];
        let Some(end) = after_open.find('}') else {
            anyhow::bail!("{field_name} has an unterminated '{{' in template {template:?}");
        };
        let name = &after_open[..end];
        if !VALID_PLACEHOLDERS.contains(&name) {
            anyhow::bail!(
                "{field_name} uses unknown placeholder {{{name}}} in template {template:?} -- valid placeholders are {{h1}}, {{h2}}, {{page}}, {{total_pages}}, {{title}}, {{author}}"
            );
        }
        rest = &after_open[end + 1..];
    }
    Ok(())
}

impl HeaderFooterStyle {
    pub fn validate(&self, section_name: &str) -> anyhow::Result<()> {
        for (zone_set_name, zones) in [("uniform", &self.uniform), ("odd", &self.odd), ("even", &self.even)] {
            validate_template(&zones.left, &format!("[{section_name}.{zone_set_name}] left"))?;
            validate_template(&zones.center, &format!("[{section_name}.{zone_set_name}] center"))?;
            validate_template(&zones.right, &format!("[{section_name}.{zone_set_name}] right"))?;
        }
        Ok(())
    }
}
