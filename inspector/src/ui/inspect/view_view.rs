use hilen::{
    inspect::protocol::ui::ViewRepr,
    refs::Weak,
    ui::{
        GRAY, Label, Setup, TURQUOISE, TextAlignment::Left, UIManager, ViewData, ViewFrame, ViewSubviews,
        ViewTouch, WHITE, view,
    },
};
use parking_lot::Mutex;

use crate::ui::VIEW_SELECTED;

pub(crate) static SHRINK_SCALE: Mutex<f32> = Mutex::new(0.2);

#[view]
pub struct ViewView {
    repr: Weak<ViewRepr>,

    #[init]
    label: Label,
}

impl Setup for ViewView {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.label.set_text_size(10).set_alignment(Left).place().lrt(0).h(10);
        self.enable_touch();

        self.outline(TURQUOISE);
    }

    fn on_selection_changed(self: Weak<Self>, selected: bool) {
        self.set_color(if selected { GRAY } else { WHITE });
        VIEW_SELECTED.trigger(self.repr);
    }
}

impl ViewView {
    pub fn set_repr(mut self: Weak<Self>, scale: f32, repr: Weak<ViewRepr>) {
        let shrink_scale = *SHRINK_SCALE.lock();

        self.label.set_text(format!("{} {}", repr.label, &repr.id[..5]));

        let frame = (
            repr.frame.x() * scale * shrink_scale / UIManager::scale(),
            repr.frame.y() * scale * shrink_scale / UIManager::scale(),
            repr.frame.width() * scale * shrink_scale / UIManager::scale(),
            repr.frame.height() * scale * shrink_scale / UIManager::scale(),
        );

        self.set_frame(frame);

        for sub in &repr.subviews {
            let view = self.add_view::<ViewView>();
            view.set_repr(scale, sub.weak());
        }

        self.repr = repr;
    }
}
