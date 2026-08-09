use refs::Weak;
use vents::Event;

use crate::{
    self as test_engine,
    gm::color::{CLEAR, Color},
    ui::{
        Anchor::Left, Container, DynamicColor, Label, Setup, TextAlignment, ToLabel, ViewData, ViewSubviews,
        view, view::ViewTouch,
    },
};

const RING_COLOR: DynamicColor =
    DynamicColor::new(Color::rgb(0.557, 0.557, 0.576), Color::rgb(0.443, 0.443, 0.467));

const DOT_COLOR: DynamicColor = DynamicColor::new(Color::rgb(0.0, 0.478, 1.0), Color::rgb(0.039, 0.518, 1.0));

const RING_DIAMETER: f32 = 20.0;
const DOT_DIAMETER: f32 = 10.0;
const RING_BORDER: f32 = 2.0;

/// Left aligned label text is already indented by `alignment_margin`, so
/// this rides on top of that rather than being the whole visual gap.
const RING_TO_LABEL: f32 = 4.0;

/// One option inside a [`RadioGroup`]. A ring that fills with a dot when
/// it is the selected one, and the option text beside it.
#[view]
pub struct RadioOption {
    pub tapped: Event<()>,

    #[init]
    ring:  Container,
    dot:   Container,
    label: Label,
}

impl RadioOption {
    pub fn set_selected(&self, selected: bool) -> &Self {
        self.dot.set_hidden(!selected);
        self
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        self.label.set_text(text);
        self
    }

    pub fn text(&self) -> &str {
        self.label.text()
    }
}

impl Setup for RadioOption {
    fn setup(self: Weak<Self>) {
        self.enable_touch();

        self.ring
            .set_color(CLEAR)
            .set_border_color(RING_COLOR)
            .set_border_width(RING_BORDER)
            .set_corner_radius(RING_DIAMETER / 2.0);
        self.ring.place().size(RING_DIAMETER, RING_DIAMETER).l(0).center_y();

        self.dot.set_color(DOT_COLOR).set_corner_radius(DOT_DIAMETER / 2.0);
        self.dot
            .place()
            .size(DOT_DIAMETER, DOT_DIAMETER)
            .center_y()
            .l((RING_DIAMETER - DOT_DIAMETER) / 2.0);
        self.dot.set_hidden(true);

        self.label.set_color(CLEAR).set_alignment(TextAlignment::Left);
        self.label.place().anchor(Left, self.ring, RING_TO_LABEL).tb(0).r(0);

        self.touch().up_inside.sub(self, move || self.tapped.trigger(()));
    }
}

/// A vertical list of options where exactly one is selected. The engine
/// equivalent of an HTML radio group.
///
/// Options are tiled top to bottom and each one takes an equal share of
/// the group's height, so size the group for the number of values it
/// holds.
#[view]
pub struct RadioGroup<T: 'static> {
    values:  Vec<T>,
    options: Vec<Weak<RadioOption>>,

    selected_index: usize,

    changed: Event<T>,
}

impl<T: ToLabel + Clone + 'static> RadioGroup<T> {
    pub fn on_changed(&self, action: impl FnMut(T) + Send + 'static) {
        self.changed.val(action);
    }

    pub fn try_get_value(&self) -> Option<&T> {
        self.values.get(self.selected_index)
    }

    pub fn value(&self) -> &T {
        assert!(!self.values.is_empty());
        self.values.get(self.selected_index).unwrap()
    }

    /// The text of the option currently selected.
    pub fn text(&self) -> &str {
        self.options.get(self.selected_index).map_or("", |option| option.text())
    }

    pub fn set_values(&mut self, values: Vec<T>) {
        for mut option in self.options.drain(..) {
            option.remove_from_superview();
        }

        self.values = values;
        self.selected_index = 0;

        let this = self.weak();

        for index in 0..self.values.len() {
            let value = self.values[index].clone();
            let option = self.add_view::<RadioOption>();
            option.set_text(value);
            option.set_selected(index == 0);
            option.tapped.sub(move || this.pick(index));
            self.options.push(option);
        }
    }

    /// A user pick. Fires `changed`, unlike [`RadioGroup::set_value`].
    fn pick(mut self: Weak<Self>, index: usize) {
        if index == self.selected_index {
            return;
        }

        self.select_index(index);
        self.changed.trigger(self.values[index].clone());
    }

    fn select_index(&mut self, index: usize) {
        self.selected_index = index;

        for (position, option) in self.options.iter().enumerate() {
            option.set_selected(position == index);
        }
    }
}

impl<T: ToLabel + Clone + PartialEq + 'static> RadioGroup<T> {
    /// Points the group at `value` without firing `changed`, so restoring
    /// a selection is never mistaken for a user pick. Returns false and
    /// changes nothing when the value is not among the current ones.
    pub fn set_value(&mut self, value: &T) -> bool {
        let Some(index) = self.values.iter().position(|existing| existing == value) else {
            return false;
        };

        self.select_index(index);

        true
    }
}

impl<T: ToLabel + Clone + 'static> Setup for RadioGroup<T> {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        // The tiling rule is solved against whatever options exist at
        // layout time, so it is set once here. Setting it in `set_values`
        // would stack a duplicate per call and clearing the placer there
        // would throw away the rules that position the group itself.
        self.place().all_ver();
    }
}
