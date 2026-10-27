//! Makes an SVG that styles itself with CSS custom properties readable by `usvg`, which resolves
//! neither `var()` nor `color-mix()`, nor the `font` shorthand or `rx` as CSS properties.

use std::collections::HashMap;

/// Resolves every `var(--name[, fallback])` in `svg`. `overrides` wins over any declaration in
/// the SVG itself, so a host can set the `--ondine-*` inputs and have each fallback dropped.
/// The custom property declarations are removed from the result, along with the CSS-only
/// constructs `usvg` cannot read being rewritten to attributes it can.
pub(crate) fn resolve(svg: &str, overrides: &HashMap<String, String>) -> String {
    let mut definitions = declarations(svg);
    definitions.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
    let resolved = substitute(svg, &definitions, 0);
    let without_declarations = strip_declarations(&resolved);
    rewrite_css_only_properties(&without_declarations)
}

/// `--name:value` pairs from every `style="..."` attribute, last one winning.
fn declarations(svg: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut rest = svg;
    while let Some(at) = rest.find("style=\"") {
        rest = &rest[at + 7..];
        let end = rest.find('"').unwrap_or(rest.len());
        for declaration in rest[..end].split(';') {
            if let Some((name, value)) = declaration.split_once(':') {
                if name.starts_with("--") {
                    map.insert(name.trim().to_string(), value.trim().to_string());
                }
            }
        }
        rest = &rest[end..];
    }
    map
}

fn substitute(text: &str, definitions: &HashMap<String, String>, depth: usize) -> String {
    const MAX_DEPTH: usize = 16;
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("var(") {
        out.push_str(&rest[..at]);
        let inner_start = at + 4;
        let Some(close) = matching_paren(&rest[inner_start..]) else {
            out.push_str(&rest[at..]);
            return out;
        };
        let inner = &rest[inner_start..inner_start + close];
        let (name, fallback) = match top_level_comma(inner) {
            Some(i) => (inner[..i].trim(), Some(inner[i + 1..].trim())),
            None => (inner.trim(), None),
        };
        let value = definitions.get(name).map(String::as_str).or(fallback).unwrap_or("");
        if depth < MAX_DEPTH {
            out.push_str(&substitute(value, definitions, depth + 1));
        }
        rest = &rest[inner_start + close + 1..];
    }
    out.push_str(rest);
    out
}

/// Byte index of the `)` that closes an already-open paren, honouring nesting.
fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Drops `--name:value;` declarations from style attributes.
fn strip_declarations(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("style=\"") {
        out.push_str(&rest[..at + 7]);
        rest = &rest[at + 7..];
        let end = rest.find('"').unwrap_or(rest.len());
        let kept: Vec<&str> = rest[..end].split(';').filter(|d| !d.trim_start().starts_with("--")).collect();
        out.push_str(&kept.join(";"));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// `font:16px monospace` becomes `font-size:16px` (the family is set separately), and
/// `style="rx:4"` becomes the `rx="4"` attribute.
fn rewrite_css_only_properties(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("style=\"") {
        out.push_str(&rest[..at]);
        rest = &rest[at + 7..];
        let end = rest.find('"').unwrap_or(rest.len());
        let mut kept = Vec::new();
        let mut attributes = String::new();
        for declaration in rest[..end].split(';').filter(|d| !d.trim().is_empty()) {
            let (name, value) = declaration.split_once(':').unwrap_or((declaration, ""));
            match name.trim() {
                "font" => {
                    if let Some(size) = value.split_whitespace().find(|w| w.ends_with("px")) {
                        kept.push(format!("font-size:{size}"));
                    }
                }
                "rx" => attributes.push_str(&format!(" rx=\"{}\"", value.trim())),
                _ => kept.push(declaration.to_string()),
            }
        }
        out.push_str(attributes.trim_start());
        if !kept.is_empty() {
            if !attributes.is_empty() {
                out.push(' ');
            }
            out.push_str(&format!("style=\"{}\"", kept.join(";")));
        }
        rest = rest.get(end + 1..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_nested_variables_and_overrides() {
        let svg = r#"<svg style="--_fg:var(--ondine-fg,#000);--_l:var(--ondine-line,var(--_fg));font:16px monospace" fill="var(--_fg)"><path stroke="var(--_l)"/><rect style="rx:var(--r,3)"/></svg>"#;
        let overrides = HashMap::from([("--ondine-fg".to_string(), "#112233".to_string())]);
        let out = resolve(svg, &overrides);
        assert_eq!(out, r##"<svg style="font-size:16px" fill="#112233"><path stroke="#112233"/><rect rx="3"/></svg>"##);
    }
}
