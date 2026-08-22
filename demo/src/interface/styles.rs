use hilen::ui::{Button, Container, Style, ViewData, ViewSubviews};

use crate::interface::home_view::HomeView;

pub static BUTTON: Style = Style::new(|button| {
    button.set_color((18, 208, 255));
    button.set_corner_radius(5);
});

// A "Back" button that returns to the home dashboard. Geometry and text
// are kept exactly as the old style so the scenes that record pixels
// around it, GameView and RootLayoutView, keep passing.
pub static HAS_BACK_BUTTON: Style = Style::new(|view| {
    view.add_view::<Button>()
        .add_transition::<Container, HomeView>()
        .set_text("Back")
        .place()
        .size(100, 50)
        .t(200)
        .l(10);
});
