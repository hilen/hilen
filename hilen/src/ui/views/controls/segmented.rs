use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::color::Color,
    ui::{DynamicColor, Setup, ToolButton, ViewData, ViewSubviews, view},
};

const TRACK: DynamicColor = DynamicColor::new(Color::hex("#EBEBF0"), Color::hex("#2C2C30"));
const PICKED: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#5A5A60"));
const PICKED_TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#FFFFFF"));
const HEIGHT: f32 = 32.0;
const INSET: f32 = 2.0;

/// A row of equal segments of which exactly 1 is picked, like the picker of
/// a period or a view mode. It fills the width it is given and is 32 points
/// tall. `changed` fires only for a tap of the user, with the picked index.
#[view]
pub struct SegmentedControl {
    picked:      usize,
    segments:    Vec<Weak<ToolButton>>,
    /// The picked index after a tap of the user.
    pub changed: Event<usize>,
}

impl SegmentedControl {
    /// Replaces the segments, the first one is picked.
    pub fn set_texts<T: ToString>(&self, texts: impl IntoIterator<Item = T>) -> &Self {
        let mut this = weak_from_ref(self);
        for mut segment in this.segments.drain(..) {
            segment.remove_from_superview();
        }
        for (index, text) in texts.into_iter().enumerate() {
            let segment = this.add_view::<ToolButton>();
            segment
                .set_text(text.to_string())
                .set_fills_width(true)
                .set_centered(true)
                .set_corner_radius(HEIGHT / 2.0 - INSET);
            segment.set_on_color(PICKED).set_on_hover_color(PICKED).set_on_tint(PICKED_TEXT);
            segment.on_tap(move || this.tapped(index));
            this.segments.push(segment);
        }
        this.picked = 0;
        this.show();
        self
    }

    /// Picks a segment with no `changed`. An index past the end is ignored.
    pub fn set_picked(&self, index: usize) -> &Self {
        if index < self.segments.len() {
            weak_from_ref(self).picked = index;
            self.show();
        }
        self
    }

    pub fn picked(&self) -> usize {
        self.picked
    }

    /// The text of the picked segment, empty with no segments.
    pub fn picked_text(&self) -> &str {
        self.segments.get(self.picked).map_or("", |segment| segment.text())
    }

    fn tapped(mut self: Weak<Self>, index: usize) {
        if index == self.picked {
            return;
        }
        self.picked = index;
        self.show();
        self.changed.trigger(index);
    }

    fn show(&self) {
        for (index, segment) in self.segments.iter().enumerate() {
            segment.set_on(index == self.picked);
        }
    }
}

impl Setup for SegmentedControl {
    fn setup(self: Weak<Self>) {
        self.set_color(TRACK).set_corner_radius(HEIGHT / 2.0);
        self.place().h(HEIGHT).all_hor().all(INSET).padding(INSET);
    }
}
