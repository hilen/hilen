use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::color::{CLEAR, Color},
    ui::{DynamicColor, Setup, ToolButton, ViewData, ViewSubviews, view},
};

const OUTLINE: DynamicColor = DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#48484E"));
const ON_OUTLINE: DynamicColor = DynamicColor::new(Color::hex("#0A6CFF"), Color::hex("#5AA5FF"));
const RADIUS: f32 = 16.0;
const GAP: f32 = 8.0;

/// How many chips of a [`Chips`] can be on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipPick {
    /// Exactly 1 is on, like a radio group. A tap on the chip that is on
    /// keeps it on.
    #[default]
    One,
    /// Any number is on, none too, like a row of check boxes.
    Many,
}

/// Texts the user picks from, as rounded chips that flow left to right and
/// wrap to the next row when the width runs out. The height follows the
/// rows, so place it with a width and no height, or put it in a stack.
/// `changed` fires only for a tap of the user, with the indexes that are on.
#[view]
pub struct Chips {
    pick:        ChipPick,
    on:          Vec<bool>,
    chips:       Vec<Weak<ToolButton>>,
    size:        f32,
    /// The indexes that are on after a tap of the user, in order.
    pub changed: Event<Vec<usize>>,
}

impl Chips {
    pub fn set_pick(&self, pick: ChipPick) -> &Self {
        weak_from_ref(self).pick = pick;
        if pick == ChipPick::One && !self.on.contains(&true) && !self.on.is_empty() {
            self.set_on_indexes([0]);
        }
        self
    }

    /// Replaces the chips. With `ChipPick::One` the first is on, with
    /// `ChipPick::Many` none is.
    pub fn set_texts<T: ToString>(&self, texts: impl IntoIterator<Item = T>) -> &Self {
        let mut this = weak_from_ref(self);
        for mut chip in this.chips.drain(..) {
            chip.remove_from_superview();
        }
        let texts: Vec<String> = texts.into_iter().map(|text| text.to_string()).collect();
        this.on = vec![false; texts.len()];
        for (index, text) in texts.into_iter().enumerate() {
            let chip = this.add_view::<ToolButton>();
            chip.set_text(text)
                .set_corner_radius(RADIUS)
                .set_border_width(1)
                .set_border_color(OUTLINE);
            if this.size > 0.0 {
                chip.set_text_size(this.size);
            }
            chip.on_tap(move || this.tapped(index));
            this.chips.push(chip);
        }
        if this.pick == ChipPick::One && !this.on.is_empty() {
            self.set_on_indexes([0]);
        }
        self
    }

    pub fn set_text_size(&self, size: f32) -> &Self {
        weak_from_ref(self).size = size;
        for chip in &self.chips {
            chip.set_text_size(size);
        }
        self
    }

    /// Turns on the chips at `indexes` and every other one off, with no
    /// `changed`. An index past the end is skipped.
    pub fn set_on_indexes(&self, indexes: impl IntoIterator<Item = usize>) -> &Self {
        let mut this = weak_from_ref(self);
        let mut on = vec![false; this.on.len()];
        for index in indexes {
            if let Some(slot) = on.get_mut(index) {
                *slot = true;
                if this.pick == ChipPick::One {
                    break;
                }
            }
        }
        this.on = on;
        this.show();
        self
    }

    /// Turns on the chips with these texts, see `set_on_indexes`.
    pub fn set_on_texts<'a>(&self, texts: impl IntoIterator<Item = &'a str>) -> &Self {
        let texts: Vec<&str> = texts.into_iter().collect();
        let indexes: Vec<usize> = self
            .chips
            .iter()
            .enumerate()
            .filter(|(_, chip)| texts.contains(&chip.text()))
            .map(|(index, _)| index)
            .collect();
        self.set_on_indexes(indexes)
    }

    /// The indexes of the chips that are on, in order.
    pub fn on_indexes(&self) -> Vec<usize> {
        self.on
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .map(|(index, _)| index)
            .collect()
    }

    /// The texts of the chips that are on, in order.
    pub fn on_texts(&self) -> Vec<String> {
        self.on_indexes()
            .into_iter()
            .map(|index| self.chips[index].text().to_string())
            .collect()
    }

    /// The chip that is on of a `ChipPick::One` group.
    pub fn picked(&self) -> Option<usize> {
        self.on.iter().position(|on| *on)
    }

    pub fn chip(&self, index: usize) -> Option<Weak<ToolButton>> {
        self.chips.get(index).copied()
    }

    fn tapped(mut self: Weak<Self>, index: usize) {
        match self.pick {
            ChipPick::One => {
                if self.on[index] {
                    return;
                }
                self.on.iter_mut().for_each(|on| *on = false);
                self.on[index] = true;
            }
            ChipPick::Many => self.on[index] = !self.on[index],
        }
        self.show();
        self.changed.trigger(self.on_indexes());
    }

    fn show(&self) {
        for (chip, on) in self.chips.iter().zip(&self.on) {
            chip.set_on(*on);
            chip.set_border_color(if *on { ON_OUTLINE } else { OUTLINE });
        }
    }
}

impl Setup for Chips {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        self.place().all_wrap().all(GAP);
    }
}
