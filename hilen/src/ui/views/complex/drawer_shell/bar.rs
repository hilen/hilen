use crate::{
    self as hilen,
    deps::refs::Weak,
    ui::{
        Anchor, Container, Label, Setup, ToolButton, ViewAccessibility, ViewData,
        images::MENU_SVG,
        view,
        views::complex::drawer_shell::{DRAWER_BAR_HEIGHT, DrawerColors},
    },
};

const MENU_HEIGHT: f32 = 36.0;
const TITLE_SIZE: f32 = 17.0;

/// The bar over the page of a narrow `DrawerShell`: the menu button that
/// opens the drawer, a title and a thin line under them.
#[view]
pub struct DrawerBar {
    #[init]
    pub(super) menu: ToolButton,
    title:           Label,
    line:            Container,
}

impl Setup for DrawerBar {
    fn setup(self: Weak<Self>) {
        self.menu.set_icon_data(MENU_SVG, "hilen_menu.svg");
        self.menu.set_accessibility_label("Menu");
        self.menu.place().l(6).t((DRAWER_BAR_HEIGHT - MENU_HEIGHT) / 2.0).h(MENU_HEIGHT);
        self.title.set_text_size(TITLE_SIZE);
        self.title.place().anchor(Anchor::Left, self.menu, 4).r(12).tb(0);
        self.line.place().lrb(0).h(1);
        self.paint(DrawerColors::default());
    }
}

impl DrawerBar {
    pub(super) fn set_title(self: Weak<Self>, title: &str) {
        self.title.set_text(title);
    }

    pub(super) fn paint(self: Weak<Self>, colors: DrawerColors) {
        self.set_color(colors.bar);
        self.line.set_color(colors.line);
        self.menu.set_tint(colors.text);
        self.title.set_text_color(colors.text);
    }
}
