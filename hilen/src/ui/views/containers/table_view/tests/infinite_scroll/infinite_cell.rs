use crate::{
    deps::refs::Weak,
    ui::{Label, Setup, ToLabel, ViewData, view},
    {self as hilen},
};

#[view]
pub(crate) struct InfiniteCell {
    #[init]
    label: Label,
}

impl InfiniteCell {
    pub fn set_text(self: Weak<Self>, text: impl ToLabel) -> Weak<Self> {
        self.label.set_text(text).set_text_size(32);
        self
    }
}

impl Setup for InfiniteCell {
    fn setup(self: Weak<Self>) {
        self.label
            .set_color("#5555AA55")
            .set_border_color("#33333333")
            .set_border_width(10)
            .place()
            .tb(5)
            .lr(20);
    }
}
