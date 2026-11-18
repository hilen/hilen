//! What an open list or an open calendar needs to float over its page: the
//! view its layer starts at, and a move to the end of that layer.

use crate::ui::{ViewSubviews, WeakView};

/// The view a layer starts at for `view`: the nearest one up the tree with a
/// z of its own, like a modal, or the root.
pub(crate) fn layer_root(view: WeakView) -> WeakView {
    let mut layer = *view.superview();
    while layer.is_ok() && !layer.__base_view().z_position_custom && layer.superview().is_ok() {
        layer = *layer.superview();
    }
    layer
}

/// Moves `view` to the end of the subviews of `parent` as it is, with no
/// second setup, which `add_subview` would run for a view from another
/// parent.
pub(crate) fn move_under(mut view: WeakView, mut parent: WeakView) {
    if parent.is_null() || view.superview().raw() == parent.raw() {
        return;
    }
    let own = view.take_from_superview();
    parent.subviews_mut().push(own);
    view.__manually_set_superview(parent);
}
