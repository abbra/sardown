//! Text measurement for Mermaid layout in the document's own font.
//!
//! Mermaid sizes every node and note box from measured text widths. Its built-in measurer knows
//! the metrics of a few web fonts only, so boxes come out too narrow or too wide for the typeface
//! the diagram is then drawn in. This measurer reads advances from the document's font instead.

use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use merman::svg::{
    DeterministicTextMeasurer, MeasurementProfileId, SvgEnvironment, TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity,
    TextMeasurer, TextMetrics, TextStyle, WrapMode,
};
use std::sync::Arc;

struct FontMeasurer {
    db: Arc<Database>,
    family: String,
    /// Supplies line height and line count; only the width comes from the font.
    shape: DeterministicTextMeasurer,
}

impl FontMeasurer {
    fn width(&self, text: &str, style: &TextStyle) -> Option<f64> {
        let weight = match style.font_weight.as_deref() {
            Some("bold" | "bolder") => 700,
            Some(w) => w.parse().unwrap_or(400),
            None => 400,
        };
        let families = [Family::Name(&self.family), Family::SansSerif];
        let id = self.db.query(&Query { families: &families, weight: Weight(weight), stretch: Stretch::Normal, style: Style::Normal })?;
        self.db.with_face_data(id, |data, index| {
            let face = ttf_parser::Face::parse(data, index).ok()?;
            let scale = style.font_size / f64::from(face.units_per_em());
            let widest = text
                .lines()
                .map(|line| line.chars().filter_map(|c| face.glyph_index(c)).filter_map(|g| face.glyph_hor_advance(g)).map(f64::from).sum::<f64>())
                .fold(0.0, f64::max);
            Some(widest * scale)
        })?
    }
}

impl FontMeasurer {
    /// Greedy word wrap at `max_width`, measured in the font; returns the wrapped lines.
    fn wrap(&self, text: &str, style: &TextStyle, max_width: f64) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.lines() {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.width(&candidate, style).is_some_and(|w| w > max_width) {
                    lines.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }
}

impl TextMeasurer for FontMeasurer {
    fn measure_wrapped(&self, text: &str, style: &TextStyle, max_width: Option<f64>, wrap_mode: WrapMode) -> TextMetrics {
        let Some(max_width) = max_width else { return self.measure(text, style) };
        let single = self.shape.measure_wrapped(text, style, None, wrap_mode);
        let lines = self.wrap(text, style, max_width);
        let line_height = single.height / single.line_count.max(1) as f64;
        let width = lines.iter().filter_map(|l| self.width(l, style)).fold(0.0, f64::max);
        TextMetrics { width, height: line_height * lines.len() as f64, line_count: lines.len() }
    }

    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        let mut metrics = self.shape.measure(text, style);
        if let Some(width) = self.width(text, style) {
            metrics.width = width;
        }
        metrics
    }
}

/// An SVG render environment whose Mermaid layout is measured in `family`, looked up in `db`.
pub fn svg_environment_for_font(db: Arc<Database>, family: &str) -> SvgEnvironment {
    let measurer = FontMeasurer { db, family: family.to_string(), shape: DeterministicTextMeasurer::default() };
    let identity =
        TextMeasurementProfileIdentity::new(MeasurementProfileId::new("sardown.document-font").expect("static profile id is valid"), env!("CARGO_PKG_VERSION"))
            .expect("static profile version is valid");
    let policy = TextMeasurementPolicy::uniform(TextMeasurementProfile::new(identity, Arc::new(measurer)));
    SvgEnvironment::deterministic().with_text_measurement_policy(policy)
}
