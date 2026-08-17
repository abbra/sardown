mod rescale;
mod resolve;
mod shrink;
mod split;
mod stylesheet_for_slide;

pub use rescale::rescale_slide_content;
pub use resolve::resolve_layout;
pub use shrink::layout_slide_with_shrink;
pub use split::{split_into_slides, Slide};
pub use stylesheet_for_slide::build_slide_stylesheet;
