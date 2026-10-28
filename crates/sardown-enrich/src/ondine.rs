//! The Ondine renderer for Mermaid fences, behind the `ondine` feature.

use crate::css_vars;
use sardown_style::{Color, MermaidStyle, Stylesheet};
use std::collections::HashMap;

fn hex(color: Color) -> String {
    let [r, g, b] = color.0;
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Blends `fg` over `bg`, `amount` of `fg`: the surface tint Ondine would get from `color-mix`.
fn mix(fg: Color, bg: Color, amount: f32) -> Color {
    Color(std::array::from_fn(|i| (f32::from(fg.0[i]) * amount + f32::from(bg.0[i]) * (1.0 - amount)).round() as u8))
}

/// The `--ondine-*` inputs for `style`, with unset colors derived from the stylesheet.
fn inputs(style: &Stylesheet) -> HashMap<String, String> {
    let m: &MermaidStyle = &style.mermaid;
    let text = m.text_color.unwrap_or(style.typography.body_color);
    let line = m.line_color.unwrap_or(text);
    let border = m.border_color.unwrap_or(text);
    let fill = m.node_fill.unwrap_or(m.background);
    let family = m.font_family.clone().unwrap_or_else(|| style.code_block.default.font_family.clone());
    let mut map = HashMap::new();
    let mut set = |name: &str, value: String| {
        map.insert(format!("--ondine-{name}"), value);
    };
    set("bg", hex(m.background));
    set("fg", hex(text));
    set("muted", hex(text));
    set("line", hex(line));
    set("border", hex(border));
    set("surface", hex(fill));
    set("surface-alt", hex(m.surface_alt.unwrap_or_else(|| mix(text, m.background, 0.12))));
    set("accent", hex(m.accent.unwrap_or(text)));
    set("critical", hex(m.accent.unwrap_or(text)));
    set("stroke", format!("{}px", m.stroke_width_px));
    set("radius", format!("{}", m.radius_px));
    set("font", format!("'{family}', monospace"));
    map
}

/// Renders `source` with Ondine and returns an SVG `usvg` can read, or the compiler's error
/// (whose own line/column point at the offending line inside `source`).
pub(crate) fn render(source: &str, style: &Stylesheet) -> Result<String, ondine_compiler::Error> {
    let rendered = ondine_compiler::render(source, "", 0.0)?;
    Ok(css_vars::resolve(&rendered.svg, &inputs(style)))
}
