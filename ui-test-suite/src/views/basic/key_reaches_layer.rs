use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Container, GREEN, LIGHT_GRAY, Label, Setup, TouchStack, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// `TouchStack::key_reaches` tells an app whether an input that is no
/// key, like the Back button of a mouse, should reach a view, by the rule
/// of the keymap. With an overlay layer open a view outside it is silent
/// and the overlay answers. A box is green while it answers.
#[view]
struct KeyReachesLayer {
    #[init]
    outside_title: Label,
    outside:       Container,
    overlay_title: Label,
    overlay:       Container,
    state:         Label,
}

impl KeyReachesLayer {
    /// Whether the outside box and the overlay answer, also as colors and
    /// on the status label.
    fn paint(self: Weak<Self>) -> (bool, bool) {
        let outside = TouchStack::key_reaches(self.outside.raw());
        let overlay = TouchStack::key_reaches(self.overlay.raw());
        let color = |answers: bool| if answers { GREEN } else { LIGHT_GRAY };
        self.outside.set_color(color(outside));
        self.overlay.set_color(color(overlay));
        let word = |answers: bool| if answers { "answers" } else { "is silent" };
        self.state
            .set_text(format!("outside {}, overlay {}", word(outside), word(overlay)));
        (outside, overlay)
    }
}

impl Setup for KeyReachesLayer {
    fn setup(self: Weak<Self>) {
        self.outside_title.set_frame((20, 20, 560, 30));
        self.outside_title.set_text_size(24).set_text("box outside the overlay");
        self.outside.set_frame((20, 55, 560, 100));

        self.overlay_title.set_frame((20, 180, 560, 30));
        self.overlay_title
            .set_text_size(24)
            .set_text("box that becomes the overlay layer");
        self.overlay.set_frame((20, 215, 560, 100));

        self.state.set_frame((20, 340, 560, 30));
        self.state.set_text_size(24);
        self.paint();
    }
}

impl ViewTest for KeyReachesLayer {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (outside, overlay) = from_main(move || view.paint());
        ensure!(outside && overlay, "with no layer: {outside} {overlay}");
        checkpoint("no layer, both boxes answer and are green")?;

        let (outside, overlay) = from_main(move || {
            TouchStack::push_layer(view.overlay.weak_view());
            view.paint()
        });
        ensure!(!outside && overlay, "with the layer open: {outside} {overlay}");
        checkpoint("the overlay layer is open, only the overlay answers")?;

        let (outside, overlay) = from_main(move || {
            TouchStack::pop_layer(view.overlay.weak_view());
            view.paint()
        });
        ensure!(outside && overlay, "after the layer closed: {outside} {overlay}");
        checkpoint("the layer closed, both boxes answer again")?;
        Ok(())
    }
}
