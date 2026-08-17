use md2pdf_layout::LayoutOutput;

/// Concatenates one `LayoutOutput` per slide into a single deck-wide `LayoutOutput`. Each slide's
/// own `layout_impl` call starts fresh at page 0 with an empty anchor table -- every page number
/// and anchor page is shifted by the running total of pages from earlier slides before appending,
/// the same shift arithmetic `insert_table_of_contents` uses when prepending TOC pages.
pub fn concat_slide_layouts(slides: Vec<LayoutOutput>) -> LayoutOutput {
    let mut pages = Vec::new();
    let mut images = md2pdf_layout::ImageTable::new();
    let mut diagrams = md2pdf_enrich::DiagramTable::new();
    let mut anchors = md2pdf_layout::AnchorTable::new();
    let mut page_contexts = Vec::new();
    let mut page_width_pt = 0.0;
    let mut page_height_pt = 0.0;

    for slide in slides {
        let offset = pages.len();
        for mut page in slide.pages {
            page.page_number += offset;
            pages.push(page);
        }
        for (id, mut anchor) in slide.anchors {
            anchor.page += offset;
            anchors.insert(id, anchor);
        }
        images.extend(slide.images);
        diagrams.extend(slide.diagrams);
        page_contexts.extend(slide.page_contexts);
        page_width_pt = slide.page_width_pt;
        page_height_pt = slide.page_height_pt;
    }

    LayoutOutput { pages, images, diagrams, anchors, page_contexts, page_width_pt, page_height_pt, toc_entries: Vec::new() }
}
