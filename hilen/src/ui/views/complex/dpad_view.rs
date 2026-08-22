use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::{Apply, flat::Direction},
    ui::{
        Button, Setup, UIImages, ViewCallbacks,
        view::{ViewData, ViewFrame},
    },
};

#[view]
pub struct DPadView {
    pub on_press: Event<Direction>,

    #[init]
    up:    Button,
    down:  Button,
    left:  Button,
    right: Button,
}

impl Setup for DPadView {
    fn setup(self: Weak<Self>) {
        [
            (self.up, Direction::Up, UIImages::up()),
            (self.down, Direction::Down, UIImages::down()),
            (self.left, Direction::Left, UIImages::left()),
            (self.right, Direction::Right, UIImages::right()),
        ]
        .apply(|(view, direction, image)| {
            view.set_image(image);
            view.on_tap(move || self.on_press.trigger(direction));
            view.set_corner_radius(5);
        });
    }
}

impl ViewCallbacks for DPadView {
    fn update(&mut self) {
        let width = self.width() / 3.0;
        let height = self.height() / 2.0;

        self.up.set_frame((width, 0, width, height));
        self.left.set_frame((0, height, width, height));
        self.down.set_frame((width, height, width, height));
        self.right.set_frame((width * 2.0, height, width, height));
    }
}
