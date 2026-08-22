use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::{
        Apply,
        color::{BLACK, Color},
        flat::Point,
    },
    ui::{Container, Setup, ViewData, ViewFrame, ViewSubviews},
};

#[view]
pub struct HighlightView {
    #[init]
    t: Container,
    b: Container,
    l: Container,
    r: Container,
}

impl HighlightView {}

impl Setup for HighlightView {
    fn setup(self: Weak<Self>) {
        const WIDTH: f32 = 40.0;

        self.t.place().lrt(0).h(WIDTH);
        self.b.place().lrb(0).h(WIDTH);
        self.l.place().t(0).l(0).b(0).w(WIDTH);
        self.r.place().t(0).r(0).b(0).w(WIDTH);

        self.outline(BLACK);
    }
}

impl HighlightView {
    pub fn set(&mut self, pos: impl Into<Point>, expected: Color, actual: Color) {
        self.set_size(150, 150);
        self.set_center(pos);
        [self.t, self.b, self.l].apply(|v| {
            v.set_color(expected);
        });
        self.r.set_color(actual);
    }
}
