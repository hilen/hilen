use anyhow::{Result, bail};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        LossyConvert,
        color::{BLUE, RED, WHITE},
        flat::Rect,
    },
    ui::{Label, Setup, UIManager, ViewData, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, system_input::wait_until},
    window::{Orientations, Window},
};

/// The app views fill the part of the screen the system leaves free. On a
/// phone that is the safe area, below the status bar and above the home
/// bar. The container of the app views once had the place of the safe area
/// and the size of the whole screen, so a view at `b(0)` ended below the
/// screen. The test drops the canvas, only the real screen has a safe area.
#[view]
struct AppViewsFitScreen {
    #[init]
    top:    Label,
    status: Label,
    bottom: Label,
}

impl Setup for AppViewsFitScreen {
    fn setup(self: Weak<Self>) {
        self.top.set_text("top bar, t(0)").set_text_size(28).set_text_color(WHITE);
        self.top.set_color(BLUE).place().t(0).lr(0).h(60);

        self.status.set_text("on the canvas").set_text_size(28);
        self.status.place().center().size(560, 60);

        self.bottom.set_text("bottom bar, b(0)").set_text_size(28).set_text_color(WHITE);
        self.bottom.set_color(RED).place().b(0).lr(0).h(60);
    }
}

impl ViewTest for AppViewsFitScreen {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            UIManager::root_view().clear_test_canvas();
            view.status.set_text("whole screen");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        from_main(move || fits(view))?;
        checkpoint("whole screen: both bars are fully on the screen")?;

        turn(view, Orientations::Landscape, "landscape")?;
        checkpoint("landscape: both bars are fully on the screen")?;

        turn(view, Orientations::Portrait, "portrait")?;
        checkpoint("portrait: both bars are fully on the screen")?;

        from_main(|| Window::set_orientations(Orientations::Any));

        #[cfg(all(ios, not(tvos)))]
        {
            bars(view, true, "fullscreen")?;
            checkpoint("fullscreen: the top bar is where the status bar was")?;
            bars(view, false, "status bar back")?;
        }

        Ok(())
    }
}

/// The system turns the screen with an animation, the app views have their
/// new frame a moment after the call.
fn turn(view: Weak<AppViewsFitScreen>, orientations: Orientations, text: &'static str) -> Result<()> {
    from_main(move || {
        Window::set_orientations(orientations);
        view.status.set_text(text);
    });
    wait_for_next_frame();
    wait_fits(view, move || turned(orientations))
}

/// Fullscreen takes the status bar away, so the safe area grows with no
/// turn of the screen. A phone with a notch keeps its safe area, there the
/// wait for the change runs out and the check still holds.
#[cfg(all(ios, not(tvos)))]
fn bars(view: Weak<AppViewsFitScreen>, fullscreen: bool, text: &'static str) -> Result<()> {
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };

    use log::debug;

    let before = from_main(safe_area);
    from_main(move || {
        Window::set_fullscreen(fullscreen);
        view.status.set_text(text);
    });

    let deadline = Instant::now() + Duration::from_secs(2);
    while from_main(safe_area) == before && Instant::now() < deadline {
        sleep(Duration::from_millis(20));
    }
    let after = from_main(safe_area);
    debug!("fullscreen {fullscreen}: the safe area went from {before:?} to {after:?}");
    wait_fits(view, || true)
}

/// Waits until `ready` holds and the app views fit. A frame that stays
/// wrong fails with what is wrong about it.
fn wait_fits(view: Weak<AppViewsFitScreen>, ready: impl Fn() -> bool + Send + Sync + 'static) -> Result<()> {
    let waited = wait_until("the app views to fit the screen", move || {
        ready() && fits(view).is_ok()
    });
    if waited.is_err() {
        from_main(move || fits(view))?;
    }
    waited
}

fn turned(orientations: Orientations) -> bool {
    if !cfg!(all(ios, not(tvos))) {
        return true;
    }
    let size = Window::render_size();
    match orientations {
        Orientations::Any => true,
        Orientations::Portrait => size.height > size.width,
        Orientations::Landscape => size.width > size.height,
    }
}

fn fits(view: Weak<AppViewsFitScreen>) -> Result<()> {
    let screen = UIManager::root_view().size();
    let app = *view.absolute_frame();
    let top = *view.top.absolute_frame();
    let bottom = *view.bottom.absolute_frame();

    let seen = format!(
        "the screen is {screen:?}, the app views are {app:?}, {}",
        system_says()
    );

    if top.y() < 0.0 || app.x() < 0.0 {
        bail!("the top bar at {top:?} starts outside the screen, {seen}");
    }
    if bottom.max_y() > screen.height + 0.5 || app.max_x() > screen.width + 0.5 {
        bail!("the bottom bar at {bottom:?} ends outside the screen, {seen}");
    }
    if let Some(safe) = safe_area() {
        let off = (app.x() - safe.x()).abs()
            + (app.y() - safe.y()).abs()
            + (app.width() - safe.width()).abs()
            + (app.height() - safe.height()).abs();
        if off > 1.0 {
            bail!("the app views are not the safe area, {seen}");
        }
    }
    Ok(())
}

/// The safe area the system reports now, in points. Only a phone has one.
fn safe_area() -> Option<Rect> {
    if !cfg!(all(ios, not(tvos))) {
        return None;
    }
    let window = Window::winit_window()?;
    let scale = UIManager::scale();
    let pos = window.inner_position().ok()?;
    let size = window.inner_size();
    let x: f32 = pos.x.lossy_convert();
    let y: f32 = pos.y.lossy_convert();
    let width: f32 = size.width.lossy_convert();
    let height: f32 = size.height.lossy_convert();
    Some((x / scale, y / scale, width / scale, height / scale).into())
}

fn system_says() -> String {
    match safe_area() {
        Some(safe) => format!("the system says the safe area is {safe:?}"),
        None => "this device has no safe area".to_string(),
    }
}
