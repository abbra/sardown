mod paginate;
mod shape;
mod table;
pub use paginate::layout;
pub use shape::shape_paragraph;

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

#[derive(Debug, Clone, Copy)]
pub struct PositionedGlyph {
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    pub x_advance: f32,
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
}
