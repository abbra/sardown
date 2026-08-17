use md2pdf_layout::{AnchorPosition, ImageTable, LayoutOutput, PageContext, PositionedPage};
use md2pdf_slides::concat_slide_layouts;

fn make_output(page_count: usize, anchor: Option<(&str, usize)>) -> LayoutOutput {
    let pages: Vec<PositionedPage> = (0..page_count).map(|i| PositionedPage { page_number: i, elements: Vec::new() }).collect();
    let page_contexts: Vec<PageContext> = (0..page_count)
        .map(|_| PageContext { current_h1: None, current_h2: None, is_chapter_opener: false, suppress_header: false, suppress_footer: false })
        .collect();
    let mut anchors = md2pdf_layout::AnchorTable::new();
    if let Some((id, page)) = anchor {
        anchors.insert(id.to_string(), AnchorPosition { page, x: 0.0, y: 0.0 });
    }
    LayoutOutput {
        pages,
        images: ImageTable::new(),
        diagrams: md2pdf_enrich::DiagramTable::new(),
        anchors,
        page_contexts,
        page_width_pt: 300.0,
        page_height_pt: 200.0,
        toc_entries: Vec::new(),
    }
}

#[test]
fn pages_are_renumbered_sequentially_across_slides() {
    let combined = concat_slide_layouts(vec![make_output(1, None), make_output(1, None), make_output(1, None)]);
    assert_eq!(combined.pages.len(), 3);
    assert_eq!(combined.pages[0].page_number, 0);
    assert_eq!(combined.pages[1].page_number, 1);
    assert_eq!(combined.pages[2].page_number, 2);
}

#[test]
fn an_anchor_on_a_later_slide_is_offset_by_earlier_slides_page_counts() {
    // Slide 0 has 2 pages, slide 1 has 1 page with its own anchor at its own local page 0 --
    // after concatenation that anchor must point at physical page 2, not 0.
    let combined = concat_slide_layouts(vec![make_output(2, None), make_output(1, Some(("target", 0)))]);
    assert_eq!(combined.anchors.get("target").unwrap().page, 2);
}

#[test]
fn page_contexts_and_page_dimensions_are_preserved() {
    let combined = concat_slide_layouts(vec![make_output(1, None), make_output(2, None)]);
    assert_eq!(combined.page_contexts.len(), 3);
    assert_eq!(combined.page_width_pt, 300.0);
    assert_eq!(combined.page_height_pt, 200.0);
}
