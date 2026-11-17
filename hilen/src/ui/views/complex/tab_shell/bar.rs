use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::LossyConvert,
    ui::{
        Container, ImageView, Label, Setup, TextAlignment, Tinted, UIManager, ViewCallbacks, ViewData,
        ViewSubviews, ViewTouch, view,
        views::complex::tab_shell::{TAB_BAR_HEIGHT, Tab, TabColors},
    },
};

const ICON_SIZE: f32 = 25.0;
const LABEL_SIZE: f32 = 10.0;

/// The bar of a `TabShell`, 1 entry per tab.
#[view]
pub struct TabBar {
    pub(super) tapped: Event<usize>,

    colors: TabColors,
    items:  Vec<Weak<TabItem>>,
    /// The thin line on top, a subview of the background so it draws in
    /// front of it.
    line:   Weak<Container>,

    #[init]
    background: Container,
}

impl Setup for TabBar {
    fn setup(mut self: Weak<Self>) {
        self.colors = TabColors::default();
        // Past the bottom of the bar by the home bar of a phone, so the
        // color goes to the edge of the screen. It runs in every layout, a
        // turn of the screen changes the inset.
        self.background
            .place()
            .t(0)
            .lr(0)
            .h(TAB_BAR_HEIGHT)
            .custom(|rect| rect.size.height = TAB_BAR_HEIGHT + UIManager::system_insets().bottom);
        self.line = self.background.add_view::<Container>();
        self.line.place().t(0).lr(0).h(1);
        self.paint();
    }
}

impl TabBar {
    pub(super) fn set_tabs(mut self: Weak<Self>, tabs: Vec<Tab>) {
        for mut item in self.items.drain(..) {
            item.remove_from_superview();
        }

        let count: f32 = tabs.len().lossy_convert();
        for (index, tab) in tabs.into_iter().enumerate() {
            let item = self.add_view::<TabItem>();
            item.show(index, tab, self.colors);
            let x: f32 = index.lossy_convert();
            item.place()
                .t(0)
                .h(TAB_BAR_HEIGHT)
                .relative_x(x / count)
                .relative_width(self, 1.0 / count);
            item.tapped.val(move |index| self.tapped.trigger(index));
            self.items.push(item);
        }
    }

    pub(super) fn set_colors(mut self: Weak<Self>, colors: TabColors) {
        self.colors = colors;
        self.paint();
        for item in &self.items {
            item.set_colors(colors);
        }
    }

    pub(super) fn mark(self: Weak<Self>, index: usize) {
        for item in &self.items {
            item.set_selected(item.index == index);
        }
    }

    pub(super) fn count(self: Weak<Self>) -> usize {
        self.items.len()
    }

    fn paint(self: Weak<Self>) {
        self.background.set_color(self.colors.bar);
        self.line.set_color(self.colors.line);
    }
}

#[view]
struct TabItem {
    tapped:   Event<usize>,
    index:    usize,
    icon:     String,
    colors:   TabColors,
    selected: bool,

    #[init]
    image: ImageView,
    label: Label,
}

impl Setup for TabItem {
    fn setup(self: Weak<Self>) {
        self.image.place().center_x().t(6).size(ICON_SIZE, ICON_SIZE);
        self.label.set_text_size(LABEL_SIZE).set_alignment(TextAlignment::Center);
        self.label.place().lr(0).t(33).h(12);
        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.tapped.trigger(self.index));
    }
}

impl ViewCallbacks for TabItem {
    /// The icon is an svg with its color written in, a theme change has to
    /// tint it again.
    fn theme_changed(&mut self) {
        self.tint();
    }
}

impl TabItem {
    fn show(mut self: Weak<Self>, index: usize, tab: Tab, colors: TabColors) {
        self.index = index;
        self.icon = tab.icon;
        self.label.set_text(tab.title);
        self.set_colors(colors);
    }

    fn set_colors(mut self: Weak<Self>, colors: TabColors) {
        self.colors = colors;
        self.tint();
    }

    fn set_selected(mut self: Weak<Self>, selected: bool) {
        self.selected = selected;
        self.tint();
    }

    fn tint(&self) {
        let color = if self.selected {
            self.colors.selected
        } else {
            self.colors.normal
        };
        self.label.set_text_color(color);
        if !self.icon.is_empty() {
            self.image.set_image(Tinted {
                tint: color.resolve(),
                name: self.icon.clone(),
            });
        }
    }
}
