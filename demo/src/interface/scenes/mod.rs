mod effects;
mod frosted_hud;
mod game_scene;
mod scroll_tables;
mod text_corruption;
mod text_fonts;
mod widget_gallery;

pub use effects::EffectsScene;
pub use frosted_hud::FrostedHud;
pub use game_scene::GameScene;
use hilen::{
    refs::{Weak, manage::DataManager},
    ui::{
        Button, Font, Label, Setup, Shadow, TextAlignment, UIManager, View, ViewData, ViewSubviews, WHITE,
        view,
    },
};
pub use scroll_tables::ScrollTables;
pub use text_fonts::TextFonts;
pub use widget_gallery::WidgetGallery;

use crate::interface::{
    HomeView,
    palette::{ACCENT, SURFACE, TEXT},
};

pub const HEADER_HEIGHT: f32 = 56.0;

/// A themed "Back" button pinned top-left that returns to the home
/// dashboard. For scenes that float controls over a game level.
pub fn add_back_button<T: View>(view: Weak<T>) {
    let button = view.add_view::<Button>();
    button
        .set_color(ACCENT)
        .set_text_color(WHITE)
        .set_corner_radius(10)
        .set_text("Back");
    button.on_tap(|| {
        UIManager::set_view(HomeView::new());
    });
    button.place().tl(20).size(90, 40);
}

/// A shared scene top bar: a back button on the left and a left aligned
/// title next to it, so they never overlap on narrow screens. Scenes
/// place their content below `HEADER_HEIGHT`.
#[view]
pub struct SceneHeader {
    #[init]
    back:  Button,
    title: Label,
}

impl SceneHeader {
    pub fn set_title(&self, text: &str) -> &Self {
        self.title.set_text(text);
        self
    }
}

impl Setup for SceneHeader {
    fn setup(self: Weak<Self>) {
        self.set_color(SURFACE).set_shadow(Shadow::default());

        self.back
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10)
            .set_text("Back");
        self.back.on_tap(|| {
            UIManager::set_view(HomeView::new());
        });
        self.back.place().l(12).center_y().size(72, 32);

        self.title
            .set_text_color(TEXT)
            .set_text_size(20)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.title.place().l(96).r(12).center_y().h(30);
    }
}

pub fn add_header<T: View>(view: Weak<T>, title: &str) -> Weak<SceneHeader> {
    let header = view.add_view::<SceneHeader>();
    header.set_title(title);
    header.place().t(0).lr(0).h(HEADER_HEIGHT);
    header
}
