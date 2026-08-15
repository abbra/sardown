use krilla::color::rgb::Color;
use krilla::geom::PathBuilder;
use krilla::paint::{Fill, Stroke};
use md2pdf_layout::{PathCommand, StrokeStyle};

pub fn build_path(commands: &[PathCommand]) -> krilla::geom::Path {
    let mut builder = PathBuilder::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(x, y) => builder.move_to(x, y),
            PathCommand::LineTo(x, y) => builder.line_to(x, y),
            PathCommand::CubicTo(x1, y1, x2, y2, x, y) => builder.cubic_to(x1, y1, x2, y2, x, y),
            PathCommand::Close => builder.close(),
        }
    }
    builder.finish().expect("path builder produced an empty/invalid path")
}

pub fn krilla_fill(color: [u8; 3]) -> Fill {
    Fill { paint: Color::new(color[0], color[1], color[2]).into(), ..Default::default() }
}

pub fn krilla_stroke(style: &StrokeStyle) -> Stroke {
    Stroke { paint: Color::new(style.color[0], style.color[1], style.color[2]).into(), width: style.width, ..Default::default() }
}
