use sardown_slides::resolve_layout;
use sardown_style::{SlideLayoutStyle, SlidesStyle, VerticalAlign};

fn style_with(layouts: &[(&str, SlideLayoutStyle)], default_layout: Option<&str>) -> SlidesStyle {
    let mut sheet = SlidesStyle { default_layout: default_layout.map(String::from), ..SlidesStyle::default() };
    for (name, layout) in layouts {
        sheet.layouts.insert(name.to_string(), layout.clone());
    }
    sheet
}

#[test]
fn no_slides_section_at_all_falls_back_to_the_built_in_default() {
    let sheet = SlidesStyle::default();
    let layout = resolve_layout(None, &sheet).unwrap();
    assert_eq!(layout.vertical_align, VerticalAlign::Top);
    assert_eq!(layout.background_color, None);
    assert!(!layout.suppress_header);
    assert!(!layout.suppress_footer);
}

#[test]
fn a_slide_with_no_directive_uses_default_layout() {
    let title = SlideLayoutStyle { suppress_header: true, ..SlideLayoutStyle::default() };
    let sheet = style_with(&[("title", title)], Some("title"));
    let layout = resolve_layout(None, &sheet).unwrap();
    assert!(layout.suppress_header);
}

#[test]
fn a_slides_own_directive_overrides_default_layout() {
    let default_layout = SlideLayoutStyle { suppress_header: true, ..SlideLayoutStyle::default() };
    let content_layout = SlideLayoutStyle { suppress_header: false, ..SlideLayoutStyle::default() };
    let sheet = style_with(&[("title", default_layout), ("content", content_layout)], Some("title"));
    let layout = resolve_layout(Some("content"), &sheet).unwrap();
    assert!(!layout.suppress_header);
}

#[test]
fn an_undefined_directive_layout_name_is_an_error() {
    let sheet = SlidesStyle::default();
    let err = resolve_layout(Some("nonexistent"), &sheet).unwrap_err();
    assert!(format!("{err}").contains("nonexistent"));
}

#[test]
fn an_undefined_default_layout_name_is_also_an_error() {
    let sheet = style_with(&[], Some("nonexistent"));
    let err = resolve_layout(None, &sheet).unwrap_err();
    assert!(format!("{err}").contains("nonexistent"));
}
