mod header_footer;
mod hyphenate;
mod image;
mod numbering;
mod paginate;
mod shape;
mod table;
mod toc;
pub use header_footer::{apply_asymmetric_margins, layout_with_header_footer, render_headers_footers, resolve_template};
pub use hyphenate::{insert_hyphenation_breaks, Hyphenator};
pub use image::{collect_svg_diagrams, decode_images, DecodedImage, ImageTable};
pub use numbering::format_page_number;
pub use paginate::{layout, layout_impl, LayoutOutput};
pub use shape::{shape_paragraph, shape_rich_paragraph, ShapedRun};
pub use toc::{insert_table_of_contents, TocEntry};

#[doc(hidden)]
pub mod test_support {
    pub use crate::table::column_widths;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PageGeometry {
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub margin_mm: f32,
    /// Both `Some` together enables asymmetric (two-sided binding) margins: `inner_margin_mm`
    /// sits nearest the spine (bigger, to allow for binding), alternating sides by physical page
    /// parity. Layout still runs its one pass using plain `margin_mm` as an arbitrary x-origin
    /// baseline; `header_footer::apply_asymmetric_margins` shifts every page's content
    /// afterward to its real final left edge (`inner_margin_mm` on recto pages, `outer_margin_mm`
    /// on verso) -- so only `content_width_pt` (below) needs to change during layout itself.
    /// `None`/`None` (the default) changes nothing about existing symmetric-margin output.
    pub inner_margin_mm: Option<f32>,
    pub outer_margin_mm: Option<f32>,
}

impl PageGeometry {
    /// Total horizontal margin budget (left + right). Constant across every page regardless of
    /// which physical side ends up "inner" vs "outer" on it, so line-wrapping width is identical
    /// for every page and only the x-origin needs to shift per page parity.
    pub fn horizontal_margin_budget_mm(&self) -> f32 {
        match (self.inner_margin_mm, self.outer_margin_mm) {
            (Some(inner), Some(outer)) => inner + outer,
            _ => 2.0 * self.margin_mm,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PositionedPage {
    pub page_number: usize,
    pub elements: Vec<PositionedElement>,
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    pub x_advance: f32,
    /// Byte range of this glyph's source cluster within the owning `TextRun`'s `text` field —
    /// needed to build a correct ToUnicode mapping so extracted/copied PDF text is accurate.
    pub cluster: std::ops::Range<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct AnchorPosition {
    pub page: usize,
    pub x: f32,
    pub y: f32,
}
pub type AnchorTable = std::collections::HashMap<String, AnchorPosition>;

#[derive(Debug, Clone)]
pub struct PageContext {
    pub current_h1: Option<String>,
    pub current_h2: Option<String>,
    pub is_chapter_opener: bool,
    /// Independent per-page suppression, orthogonal to `is_chapter_opener`: that flag always
    /// suppresses header and footer together (the book/single-document heuristic), whereas a
    /// caller that resolves suppression per page -- e.g. a slide deck honoring one layout's own
    /// `suppress_header`/`suppress_footer` -- needs to set either one independently. Callers that
    /// have no such per-page policy of their own (book/single-document rendering) always leave
    /// both `false`.
    pub suppress_header: bool,
    pub suppress_footer: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Debug, Clone, Copy)]
pub struct StrokeStyle {
    pub color: [u8; 3],
    pub width: f32,
}

/// Exhaustively matched outside this crate too, not just by `sardown-pdf`'s own renderer --
/// `sardown-slides::postprocess` walks every variant to compute a page's vertical content extent.
/// Adding a variant is a compile error at both sites until updated, which is the point: treat
/// that failure as a checklist, not a nuisance.
#[derive(Debug, Clone)]
pub enum PositionedElement {
    TextRun {
        x: f32,
        y: f32,
        glyphs: Vec<PositionedGlyph>,
        /// The shaped line's source text — `PositionedGlyph::cluster` indexes into this.
        text: String,
        font_id: fontdb::ID,
        size: f32,
        color: [u8; 3],
    },
    Path {
        points: Vec<PathCommand>,
        fill: Option<[u8; 3]>,
        stroke: Option<StrokeStyle>,
    },
    VectorGraphic {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        diagram_id: String,
    },
    LinkAnnotation {
        rect: Rect,
        destination: sardown_ast::LinkTarget,
    },
    RasterImage {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        image_id: String,
    },
}

/// Shifts one positioned element by `(dx, dy)` points -- shared by `apply_asymmetric_margins`
/// (horizontal-only, `dy = 0.0`) and sardown-slides' vertical-centering post-process (`dx = 0.0`),
/// so the coordinate-shifting logic for every `PositionedElement` variant lives in one place.
pub fn shift_element(element: &mut PositionedElement, dx: f32, dy: f32) {
    match element {
        PositionedElement::TextRun { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        PositionedElement::Path { points, .. } => {
            for command in points {
                match command {
                    PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                        *x += dx;
                        *y += dy;
                    }
                    PathCommand::CubicTo(x1, y1, x2, y2, x3, y3) => {
                        *x1 += dx;
                        *y1 += dy;
                        *x2 += dx;
                        *y2 += dy;
                        *x3 += dx;
                        *y3 += dy;
                    }
                    PathCommand::Close => {}
                }
            }
        }
        PositionedElement::VectorGraphic { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        PositionedElement::LinkAnnotation { rect, .. } => {
            rect.x += dx;
            rect.y += dy;
        }
        PositionedElement::RasterImage { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
    }
}
