use hilen::{
    refs::Weak,
    ui::{NumberView, Setup, ViewData, view},
};

use crate::interface::HAS_BACK_BUTTON;

#[view]
pub struct RenderView {
    #[init]
    val: NumberView,
}

impl Setup for RenderView {
    fn setup(self: Weak<Self>) {
        self.val.set_step(0.1).place().size(50, 100).bl(0);

        self.apply_style(HAS_BACK_BUTTON);
    }
}
