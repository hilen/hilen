use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::color::LIGHT_GRAY,
    ui::{
        Container, Setup, ViewFrame,
        view::{ViewData, ViewTouch},
    },
};
#[view]
pub struct CheckBox {
    on: bool,

    pub selected: Event<bool>,

    #[init]
    dot: Container,
}

impl CheckBox {
    pub fn on(&self) -> bool {
        self.on
    }

    pub fn on_change<Ret>(
        self: Weak<Self>,
        mut callback: impl FnMut(bool) -> Ret + Send + 'static,
    ) -> Weak<Self> {
        self.selected.val(move |val| {
            callback(val);
        });
        self
    }

    pub fn set_on(&mut self, on: bool) {
        self.dot.set_hidden(!on);
        self.on = on;
    }
}

impl Setup for CheckBox {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();

        self.set_on(false);

        self.set_color((39, 53, 73));
        self.set_border_width(2).set_corner_radius(10);
        self.set_border_color((70, 78, 97));

        self.dot
            .set_color((88, 148, 242))
            .set_corner_radius(4)
            .set_border_width(1.3)
            .set_border_color(LIGHT_GRAY);
        self.dot
            .place()
            .custom(move |f| {
                let side = self.size().smallest_side() * 0.42;
                f.size = (side, side).into();
            })
            .center();

        self.touch().up_inside.sub(self, move || {
            let on = !self.on;
            self.set_on(on);
            self.selected.trigger(on);
        });
    }
}
