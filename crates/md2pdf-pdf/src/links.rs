use krilla::action::{Action, LinkAction};
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::destination::{Destination, XyzDestination};
use krilla::geom::{Point, Rect as KrillaRect};
use md2pdf_ast::LinkTarget;
use md2pdf_layout::{AnchorTable, Rect};

/// Returns `None` for a dangling internal anchor (no matching heading found) rather than
/// erroring the whole render — a broken cross-reference shouldn't take down the document.
pub fn build_annotation(rect: &Rect, destination: &LinkTarget, anchors: &AnchorTable) -> Option<Annotation> {
    let krilla_rect = KrillaRect::from_xywh(rect.x, rect.y, rect.width, rect.height)?;
    let target = match destination {
        LinkTarget::ExternalUrl(url) => Target::Action(Action::Link(LinkAction::new(url.clone()))),
        LinkTarget::InternalAnchor(id) => {
            let anchor = anchors.get(id)?;
            Target::Destination(Destination::Xyz(XyzDestination::new(anchor.page, Point::from_xy(anchor.x, anchor.y))))
        }
        // Should never reach rendering -- md2pdf-book always resolves or drops CrossFileAnchor
        // before calling layout(). Degrade the same way a dangling InternalAnchor does (no
        // annotation, not a panic) rather than assume the invariant can never be violated.
        LinkTarget::CrossFileAnchor { .. } => return None,
    };
    Some(Annotation::new_link(LinkAnnotation::new(krilla_rect, target), None))
}
