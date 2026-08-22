use std::ops::{Deref, DerefMut};

use plat::Platform;
use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::{
        color::WHITE,
        flat::{Point, Size},
    },
    ui::{
        Anchor::Top, ImageView, Label, Setup, UIImages, View, ViewData, ViewFrame, ViewSubviews, ViewTouch,
    },
};

#[view]
pub struct MovableView<T: View + Default + 'static> {
    pub target_view: Weak<T>,

    began_pos:  Point,
    began_size: Size,

    #[init]
    title_label: Label,
    corner_view: ImageView,
}

impl<T: View + Default + 'static> Setup for MovableView<T> {
    fn setup(mut self: Weak<Self>) {
        self.title_label.set_color(WHITE).place().lrt(0).h(40);
        self.title_label.enable_touch();

        self.title_label.touch().began.val(move |touch| {
            self.place().clear();
            self.began_pos = touch.position;
        });

        self.title_label.touch().moved.val(move |touch| {
            let new_pos = self.frame().origin + touch.position - self.began_pos;
            self.set_position(new_pos);
        });

        self.target_view = self.add_view();
        self.target_view.place().lrb(0).anchor(Top, self.title_label, 0);

        let corner_size = if Platform::MOBILE { 50 } else { 28 };

        self.corner_view
            .set_image(UIImages::rb())
            .place()
            .size(corner_size, corner_size)
            .br(0);
        self.corner_view.enable_touch();

        self.corner_view.touch().began.val(move |touch| {
            self.place().clear();
            self.began_pos = touch.position;
            self.began_size = self.size();
        });

        self.corner_view.touch().moved.val(move |touch| {
            let new_size = self.size().into_point() + touch.position - self.began_pos;
            let new_size = new_size.clamp(100.0, 100.0).into_size();
            self.set_size(new_size.width, new_size.height);
        });
        self.corner_view.draw_on_top();
    }
}

impl<T: View + Default> MovableView<T> {
    pub fn title(&self) -> &str {
        self.title_label.text()
    }

    pub fn set_title(&mut self, title: &str) {
        self.title_label.set_text(title);
    }
}

impl<T: View + Default + 'static> Deref for MovableView<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.target_view.deref()
    }
}

impl<T: View + Default + 'static> DerefMut for MovableView<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.target_view.deref_mut()
    }
}
