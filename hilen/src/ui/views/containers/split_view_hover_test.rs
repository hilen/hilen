use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{CursorIcon, Hover, Setup, SplitView, ViewData, ViewTest, view},
    ui_test::inject_touches,
};

/// The resize cursor must survive the whole drag. A drag outruns the thin
/// handle between frames, and before the hover lock every such move
/// re-picked the view under the cursor and dropped the cursor icon.
#[view]
struct SplitHoverLock {
    #[init]
    split: SplitView,
}

impl Setup for SplitHoverLock {
    fn setup(self: Weak<Self>) {
        self.split.place().back();
        self.split.set_min_widths(100.0, 200.0, 100.0);
        self.split.set_left_width(150.0);
        self.split.set_right_width(150.0);
    }
}

impl ViewTest for SplitHoverLock {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let cursor = || from_main(Hover::cursor);

        // A headed window can open under the real mouse, whose entry
        // event hovers whatever it lands on before the first injection.
        from_main(Hover::clear);

        inject_touches("300 300 m");
        assert_eq!(cursor(), CursorIcon::Default);

        inject_touches("150 300 m");
        assert_eq!(cursor(), CursorIcon::ColResize);

        // Mid drag the cursor is far off the handle, the icon holds.
        inject_touches(
            "
            150 300 b
            500 300 m
        ",
        );
        assert_eq!(cursor(), CursorIcon::ColResize);

        // The cursor leaving the window mid drag must not drop it either.
        from_main(Hover::clear);
        assert_eq!(cursor(), CursorIcon::ColResize);

        // The release re-picks under the real cursor position.
        inject_touches("500 300 e");
        assert_eq!(cursor(), CursorIcon::Default);

        inject_touches("250 300 m");
        assert_eq!(cursor(), CursorIcon::ColResize);

        Ok(())
    }
}
