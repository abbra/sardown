use crate::Color;
use std::collections::BTreeMap;

const DEFAULT_LEVEL_SIZES_PT: [f32; 6] = [28.0, 22.0, 18.0, 16.0, 14.0, 12.0];

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct HeadingLevelStyle {
    pub size_pt: Option<f32>,
    pub color: Option<Color>,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct HeadingStyle {
    pub space_before_factor: f32,
    pub color: Color,
    pub font_family: String,
    /// Keyed by the level as a string ("1".."6"), matching TOML's `[heading.levels.1]` table
    /// syntax. Left empty by `Default` on purpose -- TOML's map deserialization *replaces* this
    /// field outright rather than merging key-by-key with any pre-populated defaults, so level
    /// fallback is resolved by hand in `resolve()` against `DEFAULT_LEVEL_SIZES_PT` instead.
    pub levels: BTreeMap<String, HeadingLevelStyle>,
}

impl Default for HeadingStyle {
    fn default() -> Self {
        HeadingStyle {
            space_before_factor: 0.8,
            color: Color([0, 0, 0]),
            font_family: "sans-serif".to_string(),
            levels: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedHeadingStyle {
    pub size_pt: f32,
    pub color: Color,
    pub font_family: String,
}

impl HeadingStyle {
    /// Layers `level`'s own override (if any) on top of `[heading]`'s own document-wide
    /// color/font_family, and on top of this crate's built-in per-level size table for `size_pt`.
    pub fn resolve(&self, level: u8) -> ResolvedHeadingStyle {
        let level_override = self.levels.get(&level.to_string());
        let size_pt = level_override.and_then(|l| l.size_pt).unwrap_or_else(|| DEFAULT_LEVEL_SIZES_PT[(level.clamp(1, 6) - 1) as usize]);
        let color = level_override.and_then(|l| l.color).unwrap_or(self.color);
        let font_family = level_override.and_then(|l| l.font_family.clone()).unwrap_or_else(|| self.font_family.clone());
        ResolvedHeadingStyle { size_pt, color, font_family }
    }
}
