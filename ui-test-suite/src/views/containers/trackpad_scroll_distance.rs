use anyhow::{Result, ensure};
use hilen::{
    refs::Weak,
    ui::{Color, Container, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{inject_touches, inject_trackpad_scroll},
};

/// A trackpad moves the content exactly as far as the fingers moved, like in
/// every other app. It once moved a quarter of the physical pixels, half the
/// distance on a 2x screen.
#[view]
struct TrackpadScrollDistance {
    #[init]
    scroll: ScrollView,
}

impl Setup for TrackpadScrollDistance {
    fn setup(self: Weak<Self>) {
        self.scroll.set_color(Color::rgb(0.85, 0.90, 0.95));
        self.scroll.place().tl(20).size(400, 400);
        let filler = self.scroll.add_view::<Container>();
        filler.place().t(1500).l(0).size(1, 1);
    }
}

impl ViewTest for TrackpadScrollDistance {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A tap puts the cursor over the scroll view.
        inject_touches("200 200 b\n200 200 e");
        for _ in 0..3 {
            inject_trackpad_scroll(-40);
        }
        let offset = view.scroll.get_scroll_content_offset();
        ensure!(
            (offset + 120.0).abs() < 0.01,
            "3 trackpad scrolls of 40 points moved the content to {offset}, not -120"
        );
        Ok(())
    }
}
