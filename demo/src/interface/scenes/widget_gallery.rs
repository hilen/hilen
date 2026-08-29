use hilen::{
    refs::Weak,
    ui::{
        Alert, BLACK, Button, CheckBox, Container, DropDown, Label, NumberView, ProgressView, ScrollView,
        Setup, Shadow, Spinner, SpinnerLockOnView, Switch, TextAlignment, TextField, View, ViewData,
        ViewSubviews, WHITE, WeakView, view,
    },
};

use crate::interface::{
    palette::{ACCENT, BG, BORDER, SURFACE, SURFACE_ALT, TEXT},
    scenes::{HEADER_HEIGHT, add_title},
};

/// A wrapped grid of tiles, one widget per tile with a caption, so every
/// common control can be seen and poked in one place. The grid scrolls,
/// so every tile stays reachable on small screens.
#[view]
pub struct WidgetGallery {
    grid:      Weak<Container>,
    spin_lock: SpinnerLockOnView,

    #[init]
    scroll: ScrollView,
}

impl Setup for WidgetGallery {
    fn setup(mut self: Weak<Self>) {
        self.scroll.place().t(HEADER_HEIGHT).lrb(0);
        self.grid = self.scroll.add_view::<Container>();
        self.grid.place().t(4).lr(28).all(16).all_wrap();

        self.add_widgets();

        add_title(self, "Views", "Every built in view, live and pokeable.");
    }
}

impl WidgetGallery {
    fn tile(self: Weak<Self>, caption: &str) -> Weak<Container> {
        let tile = self.grid.add_view::<Container>();
        tile.set_color(SURFACE)
            .set_corner_radius(16)
            .set_border_width(1)
            .set_border_color(BORDER);
        tile.place().size(168, 150);

        let cap = tile.add_view::<Label>();
        cap.set_text(caption.to_uppercase())
            .set_text_color(ACCENT)
            .set_text_size(10)
            .set_letter_spacing(1.2)
            .set_alignment(TextAlignment::Center);
        cap.place().t(8).lr(6).h(16);

        tile
    }

    fn toggle_spin(mut self: Weak<Self>, target: WeakView) {
        if self.spin_lock.is_active() {
            self.spin_lock = SpinnerLockOnView::default();
        } else {
            self.spin_lock = Spinner::start_on(target);
        }
    }

    fn add_widgets(self: Weak<Self>) {
        let button = self.tile("Button").add_view::<Button>();
        button
            .set_text("Tap me")
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(8);
        button.on_tap(|| {
            Alert::show("Hello from the widget gallery");
        });
        button.place().t(48).center_x().size(120, 40);

        self.tile("CheckBox")
            .add_view::<CheckBox>()
            .place()
            .t(48)
            .center_x()
            .size(46, 46);

        let toggle = self.tile("Switch").add_view::<Switch>();
        toggle.place().t(52).center_x().size(72, 40);

        self.tile("ProgressView")
            .add_view::<ProgressView>()
            .inc_progress(0.65)
            .place()
            .t(64)
            .center_x()
            .size(130, 14);

        let holder = self.tile("Spinner").add_view::<Container>();
        holder.place().t(38).center_x().size(60, 60);
        let mut spinner = holder.add_view::<Spinner>();
        spinner.dot_color = ACCENT.dark;
        spinner.place().back();

        let panel = self.tile("Spin on view");
        let target = panel.add_view::<Container>();
        target.set_color(BG).set_corner_radius(8);
        target.place().t(34).center_x().size(80, 56);
        let button = panel.add_view::<Button>();
        button
            .set_text("Toggle")
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(8);
        button.place().b(10).center_x().size(120, 30);
        button.on_tap(move || self.toggle_spin(target.weak_view()));

        let mut drop = self.tile("DropDown").add_view::<DropDown<&'static str>>();
        drop.set_values(vec!["One", "Two", "Three"]);
        drop.set_text_color(TEXT).set_text_size(15);
        drop.set_color(BG)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        drop.place().t(52).center_x().size(140, 36);

        let stepper = self.tile("NumberView");
        let value = stepper.add_view::<Label>();
        value
            .set_text("1")
            .set_text_color(TEXT)
            .set_text_size(15)
            .set_alignment(TextAlignment::Center);
        value.place().b(10).lr(6).h(18);
        let number = stepper.add_view::<NumberView>();
        number
            .set_chevrons()
            .set_separator_color(BORDER)
            .set_bevel(SURFACE_ALT.resolve(), BG.resolve());
        number.set_value(1).on_change(move |v| {
            value.set_text(format!("{v:.0}"));
        });
        number
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_shadow(Shadow {
                offset: (0, 2).into(),
                radius: 6.0,
                color:  BLACK.with_alpha(0.25),
            });
        number.place().t(34).center_x().size(52, 70);

        let field = self.tile("TextField").add_view::<TextField>();
        field.set_placeholder("Type here").set_text_size(15);
        field.place().t(52).center_x().size(150, 36);
    }
}
