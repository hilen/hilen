use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::color::{Color, WHITE},
    ui::{
        Container, DynamicColor, Setup, Shadow, UIColor, ViewFrame,
        view::{ViewData, ViewTouch},
    },
};

// The iOS system greens and grays, converted from their sRGB hex values
// to the linear floats the render pipeline expects.
const ON_TRACK: DynamicColor =
    DynamicColor::new(Color::rgb(0.034, 0.571, 0.1), Color::rgb(0.03, 0.638, 0.098));

const OFF_TRACK: DynamicColor =
    DynamicColor::new(Color::rgb(0.815, 0.815, 0.819), Color::rgb(0.041, 0.041, 0.047));

const KNOB_MARGIN: f32 = 2.0;

/// An iOS style toggle. A fully rounded track, green when on, gray when
/// off, with a white circular knob that sits at the side matching the
/// state.
#[view]
pub struct Switch {
    on: bool,

    off_color: Option<UIColor>,

    pub selected: Event<bool>,

    #[init]
    knob: Container,
}

impl Switch {
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

    pub fn set_on(&mut self, on: bool) -> &mut Self {
        self.on = on;
        self.set_color(if on {
            ON_TRACK.into()
        } else {
            self.off_color.unwrap_or(OFF_TRACK.into())
        });
        self.layout_knob();
        self
    }

    pub fn set_off_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.off_color = Some(color.into());
        if !self.on {
            let color = self.off_color.unwrap();
            self.set_color(color);
        }
        self
    }

    fn layout_knob(&mut self) {
        let diameter = self.height() - KNOB_MARGIN * 2.0;
        if diameter <= 0.0 {
            return;
        }
        self.set_corner_radius(self.height() / 2.0);
        self.knob.set_corner_radius(diameter / 2.0);
        let placer = self.knob.place().clear().size(diameter, diameter).t(KNOB_MARGIN);
        if self.on {
            placer.r(KNOB_MARGIN);
        } else {
            placer.l(KNOB_MARGIN);
        }
    }
}

impl Setup for Switch {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();
        self.knob.set_color(WHITE).set_shadow(Shadow::default());
        self.set_on(false);
        self.size_changed().sub(move || self.layout_knob());
        self.touch().began.sub(move || {
            let on = !self.on;
            self.set_on(on);
            self.selected.trigger(on);
        });
    }
}
