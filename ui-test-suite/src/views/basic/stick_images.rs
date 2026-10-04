use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Label, Setup, StickView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// 2 sticks side by side, each named by the label over it. The left one
/// keeps the gray pictures of the engine. The right one got its ring and its
/// knob picture from `set_images`. Proves both pictures are replaced and the
/// knob picture still follows a drag.
#[view]
struct StickImages {
    #[init]
    plain_title: Label,
    plain:       StickView,
    own_title:   Label,
    own:         StickView,
}

impl Setup for StickImages {
    fn setup(self: Weak<Self>) {
        self.plain_title.set_frame((20, 40, 280, 40));
        self.plain_title.set_text("default stick");
        self.plain.set_frame((60, 100, 200, 200));

        self.own_title.set_frame((300, 40, 280, 40));
        self.own_title.set_text("own ring and knob");
        self.own.set_frame((340, 100, 200, 200));
        self.own.set_images("test/blue.png", "test/ball.png");
    }
}

impl ViewTest for StickImages {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors("")?;

        inject_touches(
            "
            440 200 b
            500 200 m
        ",
        );
        check_colors("")?;

        inject_touches("500 200 e");
        Ok(())
    }
}
