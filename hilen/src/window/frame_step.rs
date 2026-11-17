//! A running app walked one frame at a time, for the inspector. All of it
//! runs on the main thread.

use std::sync::mpsc::{Receiver, Sender, channel};

use log::debug;

#[cfg(desktop)]
use crate::window::Window;
use crate::{deps::refs::main_lock::MainLock, gm::Clock, window::request_frame};

#[derive(Default)]
struct Stepper {
    paused:      bool,
    /// Frames still to draw.
    steps:       u32,
    /// Frames drawn since the pause.
    frame:       u64,
    done:        Option<Sender<u64>>,
    /// What the window said before the pause.
    saved_title: Option<String>,
}

static STEPPER: MainLock<Stepper> = MainLock::new();

pub(crate) fn holding() -> bool {
    let stepper = STEPPER.get_mut();
    stepper.paused && stepper.steps == 0
}

pub(crate) fn stepping() -> bool {
    let stepper = STEPPER.get_mut();
    stepper.paused && stepper.steps > 0
}

pub(crate) fn begin_frame() {
    if stepping() {
        Clock::advance_frame();
    }
}

pub(crate) fn end_frame() {
    let stepper = STEPPER.get_mut();
    if !stepper.paused || stepper.steps == 0 {
        return;
    }

    stepper.steps -= 1;
    stepper.frame += 1;
    show_frame(stepper.frame);

    if stepper.steps > 0 {
        request_frame();
        return;
    }
    if let Some(done) = stepper.done.take()
        && done.send(stepper.frame).is_err()
    {
        debug!("Nobody waits for the stepped frames");
    }
}

/// Freezes the clock and the drawing. Answers with the frames drawn since
/// the pause, 0 for a new one.
pub(crate) fn pause() -> u64 {
    let stepper = STEPPER.get_mut();
    if !stepper.paused {
        Clock::enter_stepped();
        stepper.paused = true;
        stepper.steps = 0;
        stepper.frame = 0;
        stepper.saved_title = title();
        show_frame(0);
    }
    stepper.frame
}

/// Draws the next `frames` frames of a paused app. The receiver gets the
/// frame count once the last one is drawn.
pub(crate) fn step(frames: u32) -> Result<Receiver<u64>, String> {
    let stepper = STEPPER.get_mut();
    if !stepper.paused {
        return Err("The app is not paused, send pause first".into());
    }
    if stepper.steps > 0 {
        return Err("Another step is still drawing".into());
    }
    if frames == 0 {
        return Err("Step needs at least 1 frame".into());
    }
    let (sender, receiver) = channel();
    stepper.steps = frames;
    stepper.done = Some(sender);
    request_frame();
    Ok(receiver)
}

pub(crate) fn resume() {
    let stepper = STEPPER.get_mut();
    if !stepper.paused {
        return;
    }
    Clock::exit_stepped();
    stepper.paused = false;
    stepper.steps = 0;
    stepper.done = None;
    if let Some(saved) = stepper.saved_title.take() {
        set_title(&saved);
    }
    request_frame();
}

fn show_frame(frame: u64) {
    set_title(&format!("paused, frame {frame}"));
}

#[cfg(desktop)]
fn title() -> Option<String> {
    Window::winit_window().map(winit::window::Window::title)
}

#[cfg(not(desktop))]
fn title() -> Option<String> {
    None
}

#[cfg(desktop)]
fn set_title(title: &str) {
    if let Some(window) = Window::winit_window() {
        window.set_title(title);
    }
}

// Only a desktop window has a title bar.
#[cfg(not(desktop))]
fn set_title(title: &str) {
    debug!("{title}");
}
