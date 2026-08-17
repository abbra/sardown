mod header_footer;
mod image;
mod paginate;
mod shape;
mod table;
mod toc;
pub use header_footer::{format_page_number, layout_with_header_footer, render_headers_footers, resolve_template};
pub use image::{decode_images, DecodedImage, ImageTable};
pub use paginate::{layout, layout_impl, LayoutOutput};
pub use shape::{shape_paragraph, shape_rich_paragraph, ShapedRun};
pub use toc::{insert_table_of_contents, TocEntry};

#[doc(hidden)]
pub mod test_support {
    pub use crate::table::column_widths;
}

#[derive(Debug, Clone, Copy)]
pub struct PageGeometry {
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub margin_mm: f32,
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
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
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
        destination: md2pdf_ast::LinkTarget,
    },
    RasterImage {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        image_id: String,
    },
}
