use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::{Own, Weak},
    ui::{Button, Color, PageSource, PageView, Setup, UIManager, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::step_frames,
};

use super::common::{HEIGHT, WIDTH, lift, press, slide};

static TAPS: AtomicU32 = AtomicU32::new(0);

#[view]
struct ButtonPage {
    #[init]
    button: Button,
}

impl Setup for ButtonPage {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.2, 0.47, 0.96));
        self.button.set_text("tap").set_color(Color::rgb(1.0, 0.9, 0.5));
        self.button.set_frame((100, 50, 200, 80));
        self.button.on_tap(|| {
            TAPS.fetch_add(1, Ordering::Relaxed);
        });
    }
}

/// A tap on a page still reaches a button on it, and a finger that moves
/// down first turns no page.
#[view]
struct PageViewTaps {
    #[init]
    pager: PageView,
}

impl Setup for PageViewTaps {
    fn setup(self: Weak<Self>) {
        self.pager.set_frame((0, 0, WIDTH, HEIGHT));
        self.pager.set_source(self).set_page("1");
    }
}

impl PageSource for PageViewTaps {
    fn page_beside(&self, key: &str, forward: bool) -> Option<String> {
        (key == "1" && forward).then(|| "2".to_string())
    }

    fn make_page(&mut self, _key: &str) -> Own<dyn View> {
        ButtonPage::new()
    }
}

impl ViewTest for PageViewTaps {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
        TAPS.store(0, Ordering::Relaxed);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pager = view.pager;
        from_main(Clock::enter_stepped);
        press(200.0, 90.0);
        lift(200.0, 90.0);
        ensure!(
            TAPS.load(Ordering::Relaxed) == 1,
            "a tap on the page did not reach its button"
        );

        // Down first, under the button.
        press(200.0, 160.0);
        for y in [168.0, 176.0, 184.0, 192.0, 200.0] {
            slide(y, 200.0, 206.0, 6.0);
        }
        lift(206.0, 200.0);
        step_frames(30);
        let offset = from_main(move || pager.offset());
        ensure!(
            offset.abs() < f32::EPSILON,
            "a finger that went down moved the pages to {offset}"
        );
        ensure!(
            from_main(move || pager.page() == Some("1")),
            "a finger that went down turned the page"
        );
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
