use hilen::{
    Event,
    gm::color::CLEAR,
    refs::Weak,
    ui::{ImageView, Setup, ViewData, ViewTouch, view},
    window::image::Image,
};

use crate::ui::frames::colors::{Mark, SELECTED, WAITED};

/// The points of the colored frame around the picture.
const FRAME: f32 = 3.0;

/// A small picture of a recorded frame in the strip.
#[view]
pub struct ThumbView {
    pub tapped: Event,
    mark:       Mark,
    selected:   bool,

    #[init]
    picture: ImageView,
}

impl ThumbView {
    pub fn show(self: Weak<Self>, image: Weak<Image>) {
        self.picture.set_image(image);
    }

    pub fn set_mark(mut self: Weak<Self>, mark: Mark) {
        self.mark = mark;
        self.paint();
    }

    pub fn set_selected(mut self: Weak<Self>, selected: bool) {
        self.selected = selected;
        self.paint();
    }

    fn paint(self: Weak<Self>) {
        if self.selected {
            self.set_color(SELECTED);
            return;
        }
        match self.mark {
            Mark::Waited => self.set_color(WAITED),
            Mark::Plain => self.set_color(CLEAR),
        };
    }
}

impl Setup for ThumbView {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(4);
        self.picture.place().lr(FRAME).t(FRAME).b(FRAME);
        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.tapped.trigger(()));
    }
}
