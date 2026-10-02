use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::color::{CLEAR, Color, WHITE},
    ui::{
        DynamicColor, ImageView, Setup, Shadow, UIColor, ViewFrame,
        view::{ViewData, ViewTouch},
    },
    window::image::{Image, ToImage},
};

// The iOS system greens and grays, converted from their sRGB hex values
// to the linear floats the render pipeline expects.
const ON_TRACK: DynamicColor =
    DynamicColor::new(Color::rgb(0.034, 0.571, 0.1), Color::rgb(0.03, 0.638, 0.098));

const OFF_TRACK: DynamicColor =
    DynamicColor::new(Color::rgb(0.815, 0.815, 0.819), Color::rgb(0.041, 0.041, 0.047));

const KNOB_INSET: f32 = 2.0;

/// The 2 pictures of a part of the switch, 1 per state.
struct StateImages {
    on:  Weak<Image>,
    off: Weak<Image>,
}

/// An iOS style toggle. A fully rounded track, green when on, gray when
/// off, with a white circular knob that sits at the side matching the
/// state.
///
/// Every part can be changed. `set_on_color`, `set_off_color` and
/// `set_knob_color` give it the palette of an app. `set_track_images` and
/// `set_knob_images` draw pictures in place of the flat shapes, and
/// `set_knob_inset` and `set_knob_shadow` size and lift the knob.
#[view]
pub struct Switch {
    on: bool,

    on_color:     Option<UIColor>,
    off_color:    Option<UIColor>,
    knob_color:   Option<UIColor>,
    knob_inset:   Option<f32>,
    track_images: Option<StateImages>,
    knob_images:  Option<StateImages>,

    pub selected: Event<bool>,

    #[init]
    track: ImageView,
    knob:  ImageView,
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
        self.paint();
        self.layout_knob();
        self
    }

    /// The color of the track while the switch is on.
    pub fn set_on_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.on_color = Some(color.into());
        self.paint();
        self
    }

    /// The color of the track while the switch is off.
    pub fn set_off_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.off_color = Some(color.into());
        self.paint();
        self
    }

    pub fn set_knob_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.knob_color = Some(color.into());
        self.paint();
        self
    }

    /// A picture of the whole track for each state. It fills the switch and
    /// replaces the track color, so the picture brings its own shape.
    pub fn set_track_images(&mut self, on: impl ToImage, off: impl ToImage) -> &mut Self {
        self.track_images = Some(StateImages {
            on:  on.to_image(),
            off: off.to_image(),
        });
        self.paint();
        self
    }

    /// A picture of the knob for each state. It fills the knob, cut to its
    /// circle, and replaces the knob color. The shadow stays,
    /// `set_knob_shadow` changes it.
    pub fn set_knob_images(&mut self, on: impl ToImage, off: impl ToImage) -> &mut Self {
        self.knob_images = Some(StateImages {
            on:  on.to_image(),
            off: off.to_image(),
        });
        self.paint();
        self
    }

    /// The gap between the knob and the edge of the track, 2 points when not
    /// set. A negative inset makes the knob bigger than the track.
    pub fn set_knob_inset(&mut self, inset: f32) -> &mut Self {
        self.knob_inset = Some(inset);
        self.layout_knob();
        self
    }

    /// The shadow under the knob. A shadow with radius 0 removes it.
    pub fn set_knob_shadow(&mut self, shadow: Shadow) -> &mut Self {
        self.knob.set_shadow(shadow);
        self
    }

    fn paint(&mut self) {
        if let Some(images) = &self.track_images {
            let image = if self.on { images.on } else { images.off };
            self.track.set_image(image);
            self.track.set_hidden(false);
            self.set_color(CLEAR);
        } else {
            let color = if self.on {
                self.on_color.unwrap_or(ON_TRACK.into())
            } else {
                self.off_color.unwrap_or(OFF_TRACK.into())
            };
            self.set_color(color);
        }

        if let Some(images) = &self.knob_images {
            let image = if self.on { images.on } else { images.off };
            self.knob.set_image(image);
            self.knob.set_color(CLEAR);
        } else {
            self.knob.set_color(self.knob_color.unwrap_or(WHITE.into()));
        }
    }

    fn layout_knob(&mut self) {
        let inset = self.knob_inset.unwrap_or(KNOB_INSET);
        let diameter = self.height() - inset * 2.0;
        if diameter <= 0.0 {
            return;
        }
        self.set_corner_radius(self.height() / 2.0);
        self.knob.set_corner_radius(diameter / 2.0);
        let placer = self.knob.place().clear().size(diameter, diameter).t(inset);
        if self.on {
            placer.r(inset);
        } else {
            placer.l(inset);
        }
    }
}

impl Setup for Switch {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();
        self.track.set_hidden(true);
        self.track.place().back();
        self.knob.set_shadow(Shadow::default());
        self.set_on(false);
        self.size_changed().sub(move || self.layout_knob());
        self.touch().began.sub(move || {
            let on = !self.on;
            self.set_on(on);
            self.selected.trigger(on);
        });
    }
}
