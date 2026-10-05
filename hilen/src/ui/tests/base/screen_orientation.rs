use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::checkpoint,
    window::{Orientations, Window},
};

/// Asks for landscape, for portrait and for every way again. Proves the
/// value reads back on every platform, and on an iPhone that the screen
/// really turns and that fullscreen there is a state the app can enter,
/// read and leave.
#[view]
struct ScreenOrientation {
    #[init]
    status: Label,
}

impl Setup for ScreenOrientation {
    fn setup(self: Weak<Self>) {
        self.status.set_text("every way").place().tl(20).size(560, 60);
    }
}

impl ViewTest for ScreenOrientation {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(
            from_main(Window::orientations) == Orientations::Any,
            "a fresh app lets the screen turn every way"
        );

        ask(view, Orientations::Landscape, "landscape")?;
        checkpoint("landscape: a phone shows the app on its side")?;

        ask(view, Orientations::Portrait, "portrait")?;
        checkpoint("portrait: a phone shows the app upright")?;

        ask(view, Orientations::Any, "every way")?;

        #[cfg(all(ios, not(tvos)))]
        {
            ensure!(
                !from_main(Window::is_fullscreen),
                "a phone app starts with its status bar"
            );
            from_main(move || {
                Window::set_fullscreen(true);
                view.status.set_text("fullscreen");
            });
            ensure!(
                from_main(Window::is_fullscreen),
                "fullscreen must read back on a phone"
            );
            checkpoint("fullscreen: no status bar")?;
            from_main(move || {
                Window::set_fullscreen(false);
                view.status.set_text("every way");
            });
            ensure!(
                !from_main(Window::is_fullscreen),
                "leaving fullscreen must read back on a phone"
            );
        }

        Ok(())
    }
}

fn ask(view: Weak<ScreenOrientation>, orientations: Orientations, text: &'static str) -> Result<()> {
    from_main(move || {
        Window::set_orientations(orientations);
        view.status.set_text(text);
    });
    ensure!(
        from_main(Window::orientations) == orientations,
        "the orientations must read back as {orientations:?}"
    );
    #[cfg(all(ios, not(tvos)))]
    wait_for_turn(orientations)?;
    Ok(())
}

/// The system turns the screen with an animation, the window has its new
/// size a moment after the call.
#[cfg(all(ios, not(tvos)))]
fn wait_for_turn(orientations: Orientations) -> Result<()> {
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };

    let fits = move || {
        let size = Window::inner_size();
        match orientations {
            Orientations::Any => true,
            Orientations::Portrait => size.height > size.width,
            Orientations::Landscape => size.width > size.height,
        }
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while !from_main(fits) {
        ensure!(
            Instant::now() < deadline,
            "the screen did not turn to {orientations:?}, the window is {:?}",
            from_main(Window::inner_size)
        );
        sleep(Duration::from_millis(50));
    }
    Ok(())
}
