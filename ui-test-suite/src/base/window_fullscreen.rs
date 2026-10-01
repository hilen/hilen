use anyhow::{Result, ensure};
use hilen::{
    Window,
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// A player page: it enters and leaves fullscreen and hears about every
/// change. Proves `set_fullscreen` moves the state, `on_fullscreen` fires
/// once per change with the new state, and asking for the state the window
/// is already in is nothing. With a real window the window itself goes
/// fullscreen and back, the headless lane has no window and pins the state
/// and the event.
#[view]
struct WindowFullscreen {
    /// Every `on_fullscreen` value, in order.
    changes: Vec<bool>,
    #[init]
    title:   Label,
    state:   Label,
}

impl Setup for WindowFullscreen {
    fn setup(mut self: Weak<Self>) {
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("the window enters and leaves fullscreen");
        self.state.set_frame((20, 100, 560, 40));
        self.state.set_text("fullscreen: false");
        Window::on_fullscreen().val(self, move |fullscreen| {
            self.changes.push(fullscreen);
            self.state.set_text(format!("fullscreen: {fullscreen}"));
        });
    }
}

impl ViewTest for WindowFullscreen {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || from_main(move || (Window::is_fullscreen(), view.changes.clone()));
        ensure!(state() == (false, vec![]), "a fresh window is not fullscreen");

        from_main(|| Window::set_fullscreen(true));
        ensure!(
            state() == (true, vec![true]),
            "entering fires once, got {:?}",
            state()
        );
        checkpoint("fullscreen: true")?;

        // Entering twice is one change.
        from_main(|| Window::set_fullscreen(true));
        ensure!(
            state() == (true, vec![true]),
            "entering again is nothing, got {:?}",
            state()
        );

        from_main(|| Window::set_fullscreen(false));
        ensure!(
            state() == (false, vec![true, false]),
            "leaving fires once, got {:?}",
            state()
        );
        checkpoint("fullscreen: false, the window is back")?;
        Ok(())
    }
}
