use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::{color::BLACK, flat::Point},
    ui::{Container, Label, Setup, UIEvent, ViewData, ViewFrame, ViewTouch},
};

#[view]
pub struct PositionView {
    began_pos: Point,

    pub moved: UIEvent<Point>,

    pub additional_label: Option<String>,

    pub position: Point,

    #[init]
    dot:   Container,
    label: Label,
}

impl PositionView {
    pub fn update_label(&mut self) {
        self.position = (self.frame().origin.x, self.frame().origin.y).into();

        let mut label = format!("{:.0} - {:.0}", self.frame().origin.x, self.frame().origin.y);

        if let Some(additional_label) = &self.additional_label {
            label = format!("{additional_label} {label}");
        }

        self.label.set_text(label);
    }
}

impl Setup for PositionView {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();
        self.place().size(250, 50);
        self.dot.set_color(BLACK).place().tl(-5).size(10, 10);
        self.label.set_text("Move me").place().back();
        self.touch().began.val(move |touch| {
            self.began_pos = touch.position;
        });
        self.touch().moved.val(move |touch| {
            let new_pos = self.frame().origin + touch.position - self.began_pos;

            self.set_position(new_pos);
            self.update_label();
            self.moved.trigger(new_pos);
        });
    }
}
