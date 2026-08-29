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
    ui::{Button, Font, Label, Setup, TextAlignment, UIManager, View, ViewData, ViewSubviews, WHITE, view},
};
pub use scroll_tables::ScrollTables;
pub use text_fonts::TextFonts;
pub use widget_gallery::WidgetGallery;

use crate::interface::{
    HomeView,
    palette::{ACCENT, TEXT, TEXT_DIM},
};

pub const HEADER_HEIGHT: f32 = 84.0;

/// A themed "Back" button pinned top-left that returns to the home
/// shell. For full screen scenes that draw a level behind the UI.
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

/// The title block of an in place page: a big title and one line of
/// detail under it. Pages place their content below `HEADER_HEIGHT`.
#[view]
pub struct PageTitle {
    #[init]
    title:  Label,
    detail: Label,
}

impl PageTitle {
    pub fn set_content(&self, title: &str, detail: &str) -> &Self {
        self.title.set_text(title);
        self.detail.set_text(detail);
        self
    }
}

impl Setup for PageTitle {
    fn setup(self: Weak<Self>) {
        self.title
            .set_text_color(TEXT)
            .set_text_size(28)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.title.place().l(28).t(22).r(20).h(34);

        self.detail
            .set_text_color(TEXT_DIM)
            .set_text_size(14)
            .set_alignment(TextAlignment::Left);
        self.detail.place().l(28).t(56).r(20).h(20);
    }
}

pub fn add_title<T: View>(view: Weak<T>, title: &str, detail: &str) -> Weak<PageTitle> {
    let header = view.add_view::<PageTitle>();
    header.set_content(title, detail);
    header.place().t(0).lr(0).h(HEADER_HEIGHT);
    header
}
