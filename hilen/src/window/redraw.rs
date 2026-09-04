use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not_wasm)]
use parking_lot::Mutex;
#[cfg(not_wasm)]
use winit::event_loop::{ControlFlow, EventLoopProxy};

#[cfg(all(not_wasm, not(target_os = "ios")))]
use crate::deps::hreads::is_main_thread;
#[cfg(not_wasm)]
use crate::window::app_handler::UserEvent;

/// A frame is pending. Set by any change that has to reach the screen, cleared
/// once per rendered frame. Starts true so the very first frame always draws.
static NEEDS_REDRAW: AtomicBool = AtomicBool::new(true);

/// Wakes the winit loop when it sleeps in `ControlFlow::Wait`. Sending a user
/// event is the winit blessed way to wake the loop from any thread. It stays
/// `None` in headless, which renders every iteration and ignores the flag.
///
/// Wasm never has one. It is single threaded and browser driven, the loop polls
/// every iteration and there is no other thread to wake it from. The proxy type
/// is also not `Sync` there, so a static holding it would not even compile.
#[cfg(not_wasm)]
static WAKE_PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);

#[cfg(not_wasm)]
pub(crate) fn set_wake_proxy(proxy: EventLoopProxy<UserEvent>) {
    *WAKE_PROXY.lock() = Some(proxy);
}

/// Ask for one more rendered frame. Safe to call from any thread. Continuous
/// work like animations and levels calls this every frame to keep drawing, so
/// a screen with neither goes idle and stops burning CPU.
///
/// The wake is sent even from the main thread. On iOS `about_to_wait` runs
/// before the frame is drawn, so a `request_frame` made while drawing, like the
/// one from `commit_animations`, comes too late for the current iteration to
/// react. Without a wake the loop then sleeps and a running animation stalls
/// after one frame. The wake makes the next iteration re-check the flag and
/// keep drawing.
pub(crate) fn request_frame() {
    NEEDS_REDRAW.store(true, Ordering::Relaxed);

    #[cfg(not_wasm)]
    {
        // On desktop about_to_wait runs after the frame, so a main thread
        // request_frame is picked up this iteration and a wake is redundant.
        // Waking from the main thread there instead livelocks the loop. On iOS
        // about_to_wait runs before the frame, so a request_frame made while
        // drawing, like the one from commit_animations, is missed unless the
        // loop is woken, and a running animation then stalls after one frame.
        #[cfg(not(target_os = "ios"))]
        if is_main_thread() {
            return;
        }

        if let Some(proxy) = WAKE_PROXY.lock().as_ref()
            && proxy.send_event(UserEvent::Wake).is_err()
        {
            // The event loop already closed, the app is shutting down.
            log::trace!("wake requested after the event loop closed");
        }
    }
}

/// Consumes the pending flag. The native winit loop calls this once per
/// iteration and draws only when it returns true. Wasm never waits, so it
/// never asks.
#[cfg(not_wasm)]
pub(crate) fn take_needs_render() -> bool {
    NEEDS_REDRAW.swap(false, Ordering::Relaxed)
}

/// True while something must redraw every frame, a live animation or a loaded
/// level. The loop polls while this holds so each requested frame is delivered,
/// then sleeps in Wait once it clears. A redraw requested from `about_to_wait`
/// does not re-arm a loop already asleep in Wait, so a per-frame flag is not
/// enough to keep continuous work drawing. The presence of the work is.
/// On wasm the loop is browser driven and never sleeps, only the test
/// suite asserts through this.
#[cfg(any(not_wasm, feature = "ui-tests"))]
pub(crate) fn continuous_render_active() -> bool {
    #[cfg(feature = "level")]
    let level_running = !crate::level::LevelManager::no_level();
    #[cfg(not(feature = "level"))]
    let level_running = false;
    crate::ui::UIManager::has_live_animations() || level_running
}

/// The window is minimized, fully covered or on another desktop, so nothing
/// drawn reaches a screen. Set from winit's occluded event on the main
/// thread. A browser throttles its own frame callbacks for a hidden tab.
#[cfg(not_wasm)]
static OCCLUDED: AtomicBool = AtomicBool::new(false);

#[cfg(not_wasm)]
pub(crate) fn set_occluded(occluded: bool) {
    OCCLUDED.store(occluded, Ordering::Relaxed);
}

#[cfg(not_wasm)]
pub(crate) fn occluded() -> bool {
    OCCLUDED.load(Ordering::Relaxed)
}

/// Whether a drawn frame reaches the screen right now.
#[cfg(not_wasm)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Visibility {
    Visible,
    /// Minimized, fully covered or on another desktop.
    Covered,
    /// Covered, but a screenshot waits for one frame drawn offscreen.
    CoveredScreenshot,
}

#[cfg(not_wasm)]
pub(crate) fn visibility(screenshot_pending: bool) -> Visibility {
    match (occluded(), screenshot_pending) {
        (false, _) => Visibility::Visible,
        (true, false) => Visibility::Covered,
        (true, true) => Visibility::CoveredScreenshot,
    }
}

/// What the native loop does after an iteration. `None` sleeps in `Wait`
/// without drawing, `Some(flow)` draws a frame and goes on under `flow`.
///
/// A covered window holds every frame, continuous work included, until it
/// shows again. Animations keep their clock, so a long hold lands them at
/// the end state on the first frame back. A pending screenshot is the one
/// exception, it is answered through the offscreen path and would otherwise
/// wait until the window is uncovered.
#[cfg(not_wasm)]
pub(crate) fn frame_pacing(visibility: Visibility, continuous: bool, pending: bool) -> Option<ControlFlow> {
    match visibility {
        Visibility::Covered => return None,
        Visibility::CoveredScreenshot => return Some(ControlFlow::Wait),
        Visibility::Visible => {}
    }
    if continuous {
        return Some(ControlFlow::Poll);
    }
    pending.then_some(ControlFlow::Wait)
}

#[cfg(all(test, not_wasm))]
mod test {
    use winit::event_loop::ControlFlow;

    use super::{Visibility, frame_pacing};

    // Regression: a minimized window with a live animation used to keep the
    // loop polling at full speed, drawing frames nobody could see.
    #[test]
    fn covered_window_holds_continuous_work() {
        assert_eq!(frame_pacing(Visibility::Covered, true, true), None);
        assert_eq!(frame_pacing(Visibility::Covered, false, true), None);
        assert_eq!(frame_pacing(Visibility::Covered, false, false), None);
    }

    // A screenshot of a covered window is drawn offscreen, so the request
    // must still get its one frame, and only one, no polling.
    #[test]
    fn covered_window_still_answers_a_screenshot() {
        assert_eq!(
            frame_pacing(Visibility::CoveredScreenshot, true, true),
            Some(ControlFlow::Wait)
        );
        assert_eq!(
            frame_pacing(Visibility::CoveredScreenshot, false, false),
            Some(ControlFlow::Wait)
        );
    }

    #[test]
    fn visible_window_paces_as_before() {
        assert_eq!(
            frame_pacing(Visibility::Visible, true, false),
            Some(ControlFlow::Poll)
        );
        assert_eq!(
            frame_pacing(Visibility::Visible, true, true),
            Some(ControlFlow::Poll)
        );
        assert_eq!(
            frame_pacing(Visibility::Visible, false, true),
            Some(ControlFlow::Wait)
        );
        assert_eq!(frame_pacing(Visibility::Visible, false, false), None);
    }
}
