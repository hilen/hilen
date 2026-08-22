use hilen::{
    inspect::protocol::ui::ViewRepr,
    refs::{Own, Weak},
    ui::{ViewSubviews, view},
};

use crate::ui::inspect::view_view::ViewView;

#[view]
pub struct UIRepresentView {
    scale: f32,
    repr:  Own<ViewRepr>,
}

impl UIRepresentView {
    pub fn reload(self: Weak<Self>) {
        self.remove_all_subviews();
        let view = self.add_view::<ViewView>();
        view.set_repr(self.scale, self.repr.weak());
    }

    pub fn set_root(mut self: Weak<Self>, scale: f32, repr: Own<ViewRepr>) {
        self.scale = scale;
        self.repr = repr;
        self.reload();
    }

    pub fn repr(self: Weak<Self>) -> Weak<ViewRepr> {
        self.repr.weak()
    }
}
