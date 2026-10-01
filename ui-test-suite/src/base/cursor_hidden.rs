use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Cursor, Label, Setup, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// A player page hides the mouse pointer after a few still seconds and
/// shows it again when the mouse moves. Proves `hide` and `show` move the
/// state with the mouse left free, not captured, and a capture on top of a
/// hidden pointer gives a hidden pointer back when it is released. With a
/// real window the pointer over it disappears, the headless lane has no
/// pointer and pins the state.
#[view]
struct CursorHidden {
    #[init]
    title: Label,
    state: Label,
}

impl CursorHidden {
    fn describe(self: Weak<Self>) {
        self.state.set_text(format!(
            "hidden: {}, captured: {}",
            Cursor::hidden(),
            Cursor::captured()
        ));
    }
}

impl Setup for CursorHidden {
    fn setup(self: Weak<Self>) {
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("the pointer hides without a capture");
        self.state.set_frame((20, 100, 560, 40));
        self.describe();
    }
}

impl ViewTest for CursorHidden {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || {
            from_main(move || {
                view.describe();
                (Cursor::hidden(), Cursor::captured())
            })
        };
        ensure!(state() == (false, false), "a fresh pointer shows and is free");

        from_main(Cursor::hide);
        ensure!(
            state() == (true, false),
            "hide does not capture, got {:?}",
            state()
        );
        checkpoint("hidden: true, the mouse still moves freely")?;

        // A capture on top of a hidden pointer, the release keeps it hidden.
        from_main(Cursor::capture);
        ensure!(state() == (true, true), "a hidden pointer can be captured");
        from_main(Cursor::release);
        ensure!(
            state() == (true, false),
            "the release leaves the pointer hidden, got {:?}",
            state()
        );

        from_main(Cursor::show);
        ensure!(state() == (false, false), "show brings the pointer back");
        checkpoint("hidden: false, the pointer is back")?;
        Ok(())
    }
}
