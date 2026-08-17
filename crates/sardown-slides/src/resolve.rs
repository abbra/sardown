use sardown_style::{SlideLayoutStyle, SlidesStyle};

/// Resolves one slide's own `@layout:` directive name (if any) against `[slides].default_layout`,
/// then looks the resulting name up in `[slides.layouts]`. Per the design spec, any name that is
/// *explicitly referenced* -- by a directive or by `default_layout` -- must resolve, or this
/// returns an error naming it; only when nothing is referenced at all (`slide_layout_name` is
/// `None` and `slides_style.default_layout` is `None`) does this fall back to one fully built-in
/// layout (`SlideLayoutStyle::default()`).
pub fn resolve_layout(slide_layout_name: Option<&str>, slides_style: &SlidesStyle) -> anyhow::Result<SlideLayoutStyle> {
    let name = slide_layout_name.or(slides_style.default_layout.as_deref());
    match name {
        Some(name) => slides_style
            .layouts
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("slide layout {name:?} is not defined -- add a [slides.layouts.{name}] table to the stylesheet")),
        None => Ok(SlideLayoutStyle::default()),
    }
}
