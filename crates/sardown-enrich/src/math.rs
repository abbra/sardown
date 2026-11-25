use ratex_svg::SvgOptions;

/// Typesets display-math `latex` as a standalone SVG sized for `font_size_pt` text (glyphs are
/// embedded as outlines, so no font lookup happens when the SVG is parsed). `Err` carries the
/// parser's message.
pub fn render_math_svg(latex: &str, font_size_pt: f32) -> Result<String, String> {
    let ast = ratex_parser::parse(latex).map_err(|e| e.to_string())?;
    let options = ratex_layout::LayoutOptions::default().with_style(ratex_types::MathStyle::Display).with_color(ratex_types::Color::BLACK);
    let display_list = ratex_layout::to_display_list(&ratex_layout::layout(&ast, &options));
    Ok(ratex_svg::render_to_svg(
        &display_list,
        &SvgOptions { font_size: f64::from(font_size_pt), padding: 0.0, stroke_width: 0.04, embed_glyphs: true, font_dir: String::new() },
    ))
}

#[cfg(test)]
mod tests {
    use super::render_math_svg;

    #[test]
    fn scales_with_font_size() {
        let small = render_math_svg(r"\frac{a}{b}", 10.0).unwrap();
        let large = render_math_svg(r"\frac{a}{b}", 20.0).unwrap();
        let width = |svg: &str| {
            let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
            tree.size().width()
        };
        assert!((width(&large) / width(&small) - 2.0).abs() < 0.05);
    }

    #[test]
    fn reports_bad_latex() {
        assert!(render_math_svg(r"\frac{", 10.0).is_err());
    }
}
