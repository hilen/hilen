use anyhow::{Result, ensure};
use hilen::{
    refs::Weak,
    ui::{Color, Container, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{inject_scroll, inject_touches},
};

/// A scroll view floating over a deeper one, added after it the way a
/// toast or a popover is. The wheel over the overlap goes to the one
/// drawn in front, not to the one deeper in the tree, and off the
/// overlay it still reaches the one below.
#[view]
struct WheelOverlay {
    #[init]
    host:  Host,
    front: ScrollView,
}

/// One nesting step around the back scroll view, so the tree puts it
/// deeper than the overlay.
#[view]
struct Host {
    #[init]
    back: ScrollView,
}

impl Setup for Host {
    fn setup(self: Weak<Self>) {
        self.back.set_color(Color::rgb(0.85, 0.90, 0.95));
        self.back.place().back();
        let filler = self.back.add_view::<Container>();
        filler.place().t(1500).l(0).size(1, 1);
    }
}

impl Setup for WheelOverlay {
    fn setup(self: Weak<Self>) {
        self.host.place().tl(20).size(400, 400);

        self.front.set_color(Color::rgb(0.55, 0.36, 0.96));
        self.front.place().t(220).l(20).size(400, 200);
        let filler = self.front.add_view::<Container>();
        filler.place().t(1500).l(0).size(1, 1);
    }
}

impl ViewTest for WheelOverlay {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A tap puts the cursor over the overlap.
        inject_touches("200 300 b\n200 300 e");
        for _ in 0..3 {
            inject_scroll(-40);
        }
        let front = view.front.get_scroll_content_offset();
        let back = view.host.back.get_scroll_content_offset();
        ensure!(front < 0.0, "the wheel over the overlay left it at {front}");
        ensure!(
            back.abs() < f32::EPSILON,
            "the wheel over the overlay moved the scroll view under it to {back}"
        );

        // Off the overlay the wheel reaches the one below.
        inject_touches("200 100 b\n200 100 e");
        for _ in 0..3 {
            inject_scroll(-40);
        }
        let back = view.host.back.get_scroll_content_offset();
        ensure!(
            back < 0.0,
            "the wheel off the overlay left the scroll view below at {back}"
        );

        Ok(())
    }
}
