use super::TextSelection;
use crate::{
    gm::flat::Point,
    ui::{ContextMenu, MenuItem},
};

/// The menu of selectable text, at a point on the screen: a right click
/// opens it, and so does the release of a long press on a touch screen.
pub(super) fn show(at: Point) {
    let copy = MenuItem::new("Copy", TextSelection::copy_logged);
    let copy = if TextSelection::is_empty() {
        copy.disabled()
    } else {
        copy
    };

    ContextMenu::show(
        vec![copy, MenuItem::new("Select All", TextSelection::select_all)],
        at,
    );
}
