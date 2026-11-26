use crate::{
    self as hilen,
    deps::refs::Weak,
    ui::{
        Anchor, Container, ImageView, Label, Setup, ToolButton, UIColor, ViewAccessibility, ViewData,
        images::MENU_SVG,
        view,
        views::complex::drawer_shell::{DRAWER_BAR_HEIGHT, DrawerColors},
    },
};

const MENU_HEIGHT: f32 = 36.0;
const TITLE_SIZE: f32 = 17.0;
const ICON_SIZE: f32 = 20.0;

/// The bar over the page of a narrow `DrawerShell`: the menu button that
/// opens the drawer, a title with an icon left of it when the app gives
/// one, and a thin line under them.
#[view]
pub struct DrawerBar {
    #[init]
    pub(super) menu: ToolButton,
    icon:            ImageView,
    title:           Label,
    line:            Container,
}

impl Setup for DrawerBar {
    fn setup(self: Weak<Self>) {
        self.menu.set_icon_data(MENU_SVG, "hilen_menu.svg");
        self.menu.set_accessibility_label("Menu");
        self.menu.place().l(6).t((DRAWER_BAR_HEIGHT - MENU_HEIGHT) / 2.0).h(MENU_HEIGHT);
        // A long title ends with an ellipsis at the right margin.
        self.title.set_text_size(TITLE_SIZE).set_ellipsize(true);
        self.set_title_icon(None);
        self.line.place().lrb(0).h(1);
        self.paint(DrawerColors::default());
    }
}

impl DrawerBar {
    pub(super) fn set_title(self: Weak<Self>, title: &str) {
        self.title.set_text(title);
    }

    /// The name of an SVG and its color, or none for a title that stands
    /// alone right of the menu button.
    pub(super) fn set_title_icon(self: Weak<Self>, icon: Option<(&str, UIColor)>) {
        self.icon.set_hidden(icon.is_none());
        let Some((name, tint)) = icon else {
            self.title.place().clear().anchor(Anchor::Left, self.menu, 4).r(12).tb(0);
            return;
        };
        self.icon.set_icon(name, tint);
        self.icon
            .place()
            .clear()
            .anchor(Anchor::Left, self.menu, 6)
            .center_y()
            .size(ICON_SIZE, ICON_SIZE);
        // The text of the title starts 8 points inside its label.
        self.title.place().clear().anchor(Anchor::Left, self.icon, 0).r(12).tb(0);
    }

    pub(super) fn paint(self: Weak<Self>, colors: DrawerColors) {
        self.set_color(colors.bar);
        self.line.set_color(colors.line);
        self.menu.set_tint(colors.text);
        self.title.set_text_color(colors.text);
    }
}
