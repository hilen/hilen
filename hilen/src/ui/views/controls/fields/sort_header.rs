use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::color::Color,
    ui::{DynamicColor, Setup, ToolButton, ViewData, ViewFrame, ViewSubviews, view},
};

const LINE: DynamicColor = DynamicColor::new(Color::hex("#E5E5EA"), Color::hex("#3A3A3C"));
const HEIGHT: f32 = 32.0;

/// Which way a column of a [`SortHeader`] sorts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Up,
    Down,
}

/// The header row of a table whose columns sort: a button per column, the
/// sorted one with an arrow. A tap on a column sorts by it going up, a tap
/// on the sorted column turns the order. The columns take shares of the
/// width, like `distribute_ratio`. 32 points tall. `changed` fires only for
/// a tap, with the column and the order, the table sorts its rows then.
#[view]
pub struct SortHeader {
    titles:      Vec<String>,
    shares:      Vec<f32>,
    sorted:      Option<(usize, SortOrder)>,
    buttons:     Vec<Weak<ToolButton>>,
    /// The column and the order after a tap.
    pub changed: Event<(usize, SortOrder)>,
}

impl SortHeader {
    /// The columns as titles and shares of the width.
    pub fn set_columns(&self, columns: impl IntoIterator<Item = (impl ToString, f32)>) -> &Self {
        let mut this = weak_from_ref(self);
        for mut button in this.buttons.drain(..) {
            button.remove_from_superview();
        }
        let (titles, shares): (Vec<String>, Vec<f32>) = columns
            .into_iter()
            .map(|(title, share)| (title.to_string(), share.max(0.01)))
            .unzip();
        this.titles = titles;
        this.shares = shares;
        for index in 0..this.titles.len() {
            let button = this.add_view::<ToolButton>();
            button.set_fills_width(true);
            button.on_tap(move || this.tapped(index));
            this.buttons.push(button);
        }
        this.sorted = None;
        self.show();
        self.relayout();
        self
    }

    /// Marks a column as sorted with no `changed`, none for no arrow.
    pub fn set_sorted(&self, sorted: Option<(usize, SortOrder)>) -> &Self {
        weak_from_ref(self).sorted = sorted.filter(|(column, _)| *column < self.titles.len());
        self.show();
        self
    }

    pub fn sorted(&self) -> Option<(usize, SortOrder)> {
        self.sorted
    }

    /// The x and the width of a column, so the cells of the table line up.
    pub fn column_frame(&self, column: usize) -> Option<(f32, f32)> {
        self.buttons.get(column).map(|button| (button.x(), button.width()))
    }

    fn tapped(mut self: Weak<Self>, column: usize) {
        let order = match self.sorted {
            Some((sorted, SortOrder::Up)) if sorted == column => SortOrder::Down,
            _ => SortOrder::Up,
        };
        self.sorted = Some((column, order));
        self.show();
        self.changed.trigger((column, order));
    }

    fn show(&self) {
        for (index, button) in self.buttons.iter().enumerate() {
            let arrow = match self.sorted {
                Some((column, SortOrder::Up)) if column == index => " ▲",
                Some((column, SortOrder::Down)) if column == index => " ▼",
                _ => "",
            };
            button.set_text(format!("{}{arrow}", self.titles[index]));
            button.set_on(self.sorted.is_some_and(|(column, _)| column == index));
        }
    }

    fn relayout(&self) {
        let total: f32 = self.shares.iter().sum();
        let mut x = 0.0;
        for (button, share) in self.buttons.iter().zip(&self.shares) {
            let width = self.width() * share / total;
            button.set_frame((x, 0.0, width, HEIGHT));
            x += width;
        }
    }
}

impl Setup for SortHeader {
    fn setup(self: Weak<Self>) {
        self.set_border_color(LINE).set_border_width(1);
        self.place().h(HEIGHT);
        self.size_changed().sub(move || self.relayout());
    }
}
