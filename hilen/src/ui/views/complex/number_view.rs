use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        CheckedSub, MyAdd, ToF32,
        color::{CLEAR, Color, LIGHT_GRAY},
    },
    ui::{Button, Container, Setup, Style, ToLabel, UIColor, UIImages, view::ViewData},
    window::image::NoImage,
};

#[view]
pub struct NumberView {
    #[educe(Default = 1.0)]
    value:    f32,
    #[educe(Default = 1.0)]
    pub step: f32,
    #[educe(Default = f32::MIN)]
    min:      f32,

    on_change_event: Event<f32>,

    up_event:   Event,
    down_event: Event,

    #[init]
    up:   Button,
    down: Button,

    separator: Container,
}

impl Setup for NumberView {
    fn setup(self: Weak<Self>) {
        self.up.set_image(UIImages::up()).set_color(CLEAR);
        self.up.on_tap(move || self.up_tap());
        self.up.place().lrt(0).relative_height(self, 0.5);

        self.down.set_image(UIImages::down()).set_color(CLEAR);
        self.down.on_tap(move || self.down_tap());
        self.down.place().lrb(0).relative_height(self, 0.5);

        self.separator.place().relative_width(self, 0.75).h(2).center();
        self.separator.set_color(LIGHT_GRAY);

        Style::apply_global(self);
    }
}

impl NumberView {
    pub fn set_labels(&mut self, up: impl ToLabel, down: impl ToLabel) -> &mut Self {
        self.up.set_image(NoImage);
        self.up.set_text(up);

        self.down.set_image(NoImage);
        self.down.set_text(down);

        self
    }
}

impl NumberView {
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&self, val: impl ToF32) -> &Self {
        weak_from_ref(self).value = val.to_f32();
        self.on_change_event.trigger(self.value);
        self
    }

    pub fn set_min(&self, min: impl ToF32) -> &Self {
        weak_from_ref(self).min = min.to_f32();
        self.set_value(self.min);
        self
    }

    pub fn set_step(&self, step: impl ToF32) -> &Self {
        weak_from_ref(self).step = step.to_f32();
        self
    }

    fn up_tap(self: Weak<Self>) {
        let val = self.value.my_add(&self.step);
        self.set_value(val);
        self.up_event.trigger(());
    }

    fn down_tap(self: Weak<Self>) {
        let val = self.value.sub_and_check(&self.step, &self.min);
        self.set_value(val.unwrap_or(0.0));
        self.down_event.trigger(());
    }

    pub fn on_change(&self, action: impl FnMut(f32) + Send + 'static) -> &Self {
        self.on_change_event.val(action);
        self
    }

    pub fn on_up(&self, action: impl FnMut() + Send + 'static) -> &Self {
        self.up_event.sub(action);
        self
    }

    pub fn on_down(&self, action: impl FnMut() + Send + 'static) -> &Self {
        self.down_event.sub(action);
        self
    }
}

impl NumberView {
    pub fn text_color(&self) -> &Color {
        self.up.text_color()
    }

    pub fn set_text_color(&self, color: impl Into<UIColor>) -> &Self {
        let color = color.into();
        self.up.set_text_color(color);
        self.down.set_text_color(color);
        self
    }

    pub fn text_size(&self) -> f32 {
        self.up.text_size()
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        self.up.set_text_size(size);
        self.down.set_text_size(size);
        self
    }
}
