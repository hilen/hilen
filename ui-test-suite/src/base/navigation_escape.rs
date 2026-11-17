use std::time::Duration;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    time::Instant,
    ui::{
        Button, Label, NamedKey, NavigationView, Setup, View, ViewController, ViewData, ViewSubviews,
        ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches},
};

#[view]
struct EscapePushed {
    #[init]
    title: Label,
}

impl Setup for EscapePushed {
    fn setup(self: Weak<Self>) {
        self.set_color("#2C3E50");
        self.title.set_text("pushed, Escape goes back").set_text_size(24);
        self.title.set_text_color(WHITE);
        self.title.place().lrt(20).h(60);
    }
}

/// Escape, the Back key of a remote, pops the screen on top of a
/// navigation stack. The first screen has nothing to go back to.
#[view]
struct NavigationEscape {
    #[init]
    title: Label,
    push:  Button,
}

impl Setup for NavigationEscape {
    fn setup(self: Weak<Self>) {
        self.set_color("#27AE60");
        self.title.set_text("home").set_text_size(24);
        self.title.place().lrt(20).h(60);

        self.push.set_text("Push").set_text_size(24);
        self.push.set_color("#F1C40F");
        self.push.place().center().size(220, 64);
        self.push.on_tap(move || {
            self.navigation().push(EscapePushed::new());
        });
    }
}

impl ViewTest for NavigationEscape {
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let depth = move || from_main(move || view.navigation().subviews().len());

        // Nothing is pushed, Escape has nothing to pop.
        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        ensure!(depth() == 1, "Escape on the first screen changed the stack");

        inject_touches(
            "
            300 300 b
            300 300 e
        ",
        );
        wait_until("push", || from_main(move || view.is_hidden()))?;
        ensure!(depth() == 2, "the push did not land");
        checkpoint("the pushed screen is up")?;

        inject_named_key(NamedKey::Escape);
        wait_until("pop", move || {
            from_main(move || !view.is_hidden() && view.navigation().subviews().len() == 1)
        })?;
        checkpoint("Escape popped it, home is back")?;

        Ok(())
    }
}

/// The bound counts a suspended gap as one frame, a browser stops rendering
/// while the window is covered and a paused run must resume, not fail.
fn wait_until(action: &str, mut done: impl FnMut() -> bool) -> Result<()> {
    let mut waited = Duration::ZERO;
    while !done() {
        ensure!(
            waited < Duration::from_secs(10),
            "{action} animation never finished"
        );
        let frame = Instant::now();
        wait_for_next_frame();
        waited += frame.elapsed().min(Duration::from_millis(100));
    }
    Ok(())
}
