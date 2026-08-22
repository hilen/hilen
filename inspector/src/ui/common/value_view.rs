use hilen::{
    Event,
    refs::Weak,
    ui::{Label, NumberView, Setup, ToLabel, ViewData, view},
};

#[view]
pub struct ValueView {
    pub on_change: Event<f32>,

    #[init]
    title:       Label,
    buttons:     NumberView,
    value_label: Label,
}

impl ValueView {
    pub fn set_title(self: Weak<Self>, title: impl ToLabel) -> Weak<Self> {
        self.title.set_text(title);
        self
    }
}

impl Setup for ValueView {
    fn setup(self: Weak<Self>) {
        self.place().distribute_ratio([1, 1, 1]);

        self.buttons.on_change(move |val| {
            self.on_change.trigger(val);
            self.value_label.set_text(val);
        });

        self.buttons.set_min(0.1);
        self.buttons.set_step(0.1);
    }
}
