use hilen::{
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    system::open_url,
    ui::{
        AlertErr, Button, Container, Font, ImageView, Label, ScrollView, Setup, TextAlignment, Theme,
        ThemeMode, UIEvents, ViewData, ViewSubviews, ViewTouch, WHITE, view,
    },
};

use crate::interface::{
    home_view::HomeView,
    page::Page,
    palette::{ACCENT, ACCENT_SOFT, BORDER, SURFACE, SURFACE_ALT, TEXT, TEXT_DIM},
};

pub const SIDEBAR_WIDTH: f32 = 240.0;
const FOOTER_HEIGHT: f32 = 140.0;
const REPO_URL: &str = "https://github.com/hilen/hilen";

/// The left column. Logo and wordmark, the page list in groups, a
/// theme picker and a GitHub badge pinned to the bottom.
#[view]
pub struct Sidebar {
    items: Vec<Weak<NavItem>>,
    modes: Vec<(ThemeMode, Weak<Button>)>,

    #[init]
    logo:   ImageView,
    brand:  Label,
    scroll: ScrollView,
    theme:  Container,
    github: GitHubBadge,
}

impl Setup for Sidebar {
    fn setup(mut self: Weak<Self>) {
        self.set_color(SURFACE).set_border_width(1).set_border_color(BORDER);

        self.logo.set_image("engine.png").set_corner_radius(8);
        self.logo.place().tl(20).size(36, 36);

        self.brand
            .set_text("Hilen")
            .set_text_color(TEXT)
            .set_text_size(24)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.brand.place().l(66).t(22).r(12).h(32);

        self.theme
            .set_color(SURFACE_ALT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.theme.place().lr(16).b(84).h(34).all_hor().all(3);
        for (mode, name) in [
            (ThemeMode::System, "Auto"),
            (ThemeMode::Light, "Light"),
            (ThemeMode::Dark, "Dark"),
        ] {
            let button = self.theme.add_view::<Button>();
            button.set_text(name).set_text_size(12).set_corner_radius(8);
            button.on_tap(move || {
                Theme::set_mode(mode);
                self.refresh_modes();
            });
            self.modes.push((mode, button));
        }
        self.refresh_modes();
        UIEvents::theme_changed().val(self, move |_| self.refresh_modes());

        self.github.place().lr(16).b(16).h(52);

        self.scroll.place().t(72).lr(0).b(FOOTER_HEIGHT);
        for (i, page) in Page::ALL.into_iter().enumerate() {
            let item = self.scroll.add_view::<NavItem>();
            item.set_page(page);
            item.place().lr(12).t(8.0 + 44.0 * i.lossy_convert()).h(40);
            self.items.push(item);
        }
        self.mark(Page::Landing);
    }
}

impl Sidebar {
    /// Highlight the open page.
    pub fn mark(self: Weak<Self>, page: Page) {
        for item in &self.items {
            item.set_selected(item.page == page);
        }
    }

    fn refresh_modes(self: Weak<Self>) {
        for (mode, button) in &self.modes {
            if *mode == Theme::mode() {
                button.set_color(ACCENT).set_text_color(WHITE);
            } else {
                button.set_color(SURFACE_ALT).set_text_color(TEXT_DIM);
            }
        }
    }
}

/// One row of the page list. The page icon, a left aligned label, a
/// hover wash and an accent state for the open page.
#[view]
struct NavItem {
    page:     Page,
    selected: bool,

    #[init]
    icon:  ImageView,
    label: Label,
}

impl NavItem {
    fn set_page(mut self: Weak<Self>, page: Page) {
        self.page = page;
        self.icon.set_image(page.icon());
        self.label.set_text(page.title());
    }

    fn set_selected(mut self: Weak<Self>, selected: bool) {
        self.selected = selected;
        self.refresh(self.is_hovered());
    }

    fn refresh(self: Weak<Self>, hovered: bool) {
        if self.selected {
            self.set_color(ACCENT_SOFT);
            self.label.set_text_color(ACCENT);
        } else if hovered {
            self.set_color(ACCENT_SOFT);
            self.label.set_text_color(TEXT);
        } else {
            self.set_color(SURFACE);
            self.label.set_text_color(TEXT);
        }
    }
}

impl Setup for NavItem {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(10);

        self.icon.place().l(12).center_y().size(22, 22);

        self.label.set_text_size(15).set_alignment(TextAlignment::Left);
        self.label.place().l(48).r(8).tb(0);

        self.enable_touch();
        self.touch().up_inside.sub(self, move || HomeView::open(self.page));

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| self.refresh(hovered));

        self.refresh(false);
    }
}

/// A rounded pill with the GitHub mark and the repo name. Opens the
/// repo in the browser. The mark is a flat black SVG, so the dark theme
/// swaps in a white copy.
#[view]
pub struct GitHubBadge {
    #[init]
    mark: ImageView,
    name: Label,
    hint: Label,
}

impl Setup for GitHubBadge {
    fn setup(self: Weak<Self>) {
        self.set_color(SURFACE)
            .set_corner_radius(12)
            .set_border_width(1)
            .set_border_color(BORDER);

        self.refresh_mark();
        UIEvents::theme_changed().val(self, move |_| self.refresh_mark());
        self.mark.place().l(12).center_y().size(26, 26);

        self.name
            .set_text("hilen/hilen")
            .set_text_color(TEXT)
            .set_text_size(14)
            .set_alignment(TextAlignment::Left);
        self.name.place().l(50).t(9).r(10).h(18);

        self.hint
            .set_text("Source code")
            .set_text_color(TEXT_DIM)
            .set_text_size(11)
            .set_alignment(TextAlignment::Left);
        self.hint.place().l(50).b(8).r(10).h(14);

        self.enable_touch();
        self.touch().up_inside.sub(self, || {
            open_url(REPO_URL).alert_err();
        });

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            if hovered {
                self.set_border_color(ACCENT).set_color(ACCENT_SOFT);
            } else {
                self.set_border_color(BORDER).set_color(SURFACE);
            }
        });
    }
}

impl GitHubBadge {
    fn refresh_mark(self: Weak<Self>) {
        self.mark.set_image(match Theme::current() {
            Theme::Light => "github.svg",
            Theme::Dark => "github_light.svg",
        });
    }
}
