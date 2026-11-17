use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Cursor, NamedKey, Point, Setup, UIManager, ViewTest, view},
    ui_test::{inject_mouse_motion, inject_named_key},
};

/// A game page: it captures the mouse, reads the raw motion and gets the
/// mouse back on Escape. The Escape that frees the mouse must not reach
/// the page's own key handler, the next one must.
#[view]
struct CursorCapture {
    /// Every `on_capture` value, in order.
    captures: Vec<bool>,
    escapes:  u32,
}

impl Setup for CursorCapture {
    fn setup(mut self: Weak<Self>) {
        Cursor::on_capture().val(self, move |captured| self.captures.push(captured));
        UIManager::keymap().add(self, NamedKey::Escape, move || self.escapes += 1);
    }
}

impl ViewTest for CursorCapture {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || from_main(move || (Cursor::captured(), view.captures.clone(), view.escapes));
        let motion = || from_main(Cursor::take_motion);

        assert_eq!(state(), (false, vec![], 0));

        // A free mouse moves the cursor, it is no game motion.
        inject_mouse_motion((10, -4));
        assert_eq!(motion(), Point::default());

        from_main(Cursor::capture);
        assert_eq!(state(), (true, vec![true], 0));

        // Capturing twice is one capture.
        from_main(Cursor::capture);
        assert_eq!(state(), (true, vec![true], 0));

        inject_mouse_motion((10, -4));
        inject_mouse_motion((5, 5));
        assert_eq!(motion(), Point::new(15.0, 1.0));
        // Taken once, the motion is gone.
        assert_eq!(motion(), Point::default());

        inject_named_key(NamedKey::Escape);
        assert_eq!(state(), (false, vec![true, false], 0));

        // The motion the page saw before the release does not linger.
        assert_eq!(motion(), Point::default());
        inject_mouse_motion((10, -4));
        assert_eq!(motion(), Point::default());

        // With the mouse free, Escape is the page's own key again.
        inject_named_key(NamedKey::Escape);
        assert_eq!(state(), (false, vec![true, false], 1));

        // Releasing a free mouse is nothing.
        from_main(Cursor::release);
        assert_eq!(state(), (false, vec![true, false], 1));

        Ok(())
    }
}
