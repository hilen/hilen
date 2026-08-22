use hilen::{
    inspect::{protocol::ui::ViewRepr, views::PlacerView},
    refs::Weak,
    ui::{Anchor::Top, Label, Setup, ViewData, view},
};

#[view]
pub struct ViewInspectorView {
    view: Weak<ViewRepr>,

    #[init]
    label: Label,
    id:    Label,

    pub placer_view: PlacerView,
}

impl Setup for ViewInspectorView {
    fn setup(self: Weak<Self>) {
        self.label.place().ltr(0).relative_height(self, 0.05);
        self.id.place().below(self.label, 0);

        self.placer_view.place().lrb(0).anchor(Top, self.id, 0);
    }
}

impl ViewInspectorView {
    pub fn set_view(mut self: Weak<Self>, view: Weak<ViewRepr>) {
        self.label.set_text(format!("Label: {}", view.label));
        self.id.set_text_size(10).set_text(view.id.clone());
        self.placer_view.set_view(view);

        self.view = view;
    }
}
