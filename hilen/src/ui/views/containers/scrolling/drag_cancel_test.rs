use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{Button, ScrollView, Setup, UIManager, ViewData, ViewSubviews, ViewTest, view},
    ui_test::inject_touches,
};

/// Drag scrolling must not tap the view the drag began on.
/// Before the fix releasing a drag over a button pressed it, because the
/// button kept the capture and moved along with the content under the finger.
#[view]
struct DragCancel {
    taps:   u32,
    button: Weak<Button>,

    #[init]
    scroll: ScrollView,
}

impl Setup for DragCancel {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_content_size((600, 1200));
        self.scroll.place().back();

        self.button = self.scroll.add_view();
        self.button.place().size(200, 100).tl(50);
        self.button.on_tap(move || self.taps += 1);
    }
}

impl ViewTest for DragCancel {
    // This test drives finger drag gestures, which scroll only with
    // drag scrolling on, the touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || from_main(move || (view.taps, view.scroll.get_scroll_content_offset()));

        inject_touches(
            "
            150 100 b
            150 100 e
        ",
        );
        assert_eq!(state(), (1, 0.0));

        // A jitter below the drag slop is still a tap and doesn't scroll.
        inject_touches(
            "
            150 100 b
            150 97  m
            150 97  e
        ",
        );
        assert_eq!(state(), (2, 0.0));

        // A real drag scrolls and the tap is cancelled.
        inject_touches(
            "
            150 100 b
            150 50  m
        ",
        );
        assert_eq!(state(), (2, -50.0));
        inject_touches("150 50 e");
        assert_eq!(from_main(move || view.taps), 2);

        Ok(())
    }
}
