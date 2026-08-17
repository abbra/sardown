mod resolve;
mod split;
mod stylesheet_for_slide;

pub use resolve::resolve_layout;
pub use split::{split_into_slides, Slide};
pub use stylesheet_for_slide::build_slide_stylesheet;
