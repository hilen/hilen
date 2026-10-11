use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not_wasm)]
use parking_lot::Mutex;
#[cfg(not_wasm)]
use plat::Platform;
#[cfg(not_wasm)]
use winit::event_loop::{ControlFlow, EventLoopProxy};

#[cfg(all(not_wasm, not_ios))]
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

/// The proxy holds a source in the run loop, a hot build that is stopped
/// gives it back.
#[cfg(hot)]
pub(crate) fn clear_wake_proxy() {
    *WAKE_PROXY.lock() = None;
}

/// Ask for one more rendered frame. Safe to call from any thread. Continuous
/// work like animations and levels calls this every frame to keep drawing, so
/// a screen with neither goes idle and stops burning CPU.
///
/// On iOS and tvOS the wake is sent even from the main thread. There
/// `about_to_wait` runs before the frame is drawn, so a `request_frame` made
/// while drawing, like the one from `commit_animations`, comes too late for
/// the current iteration to react. Without a wake the loop then sleeps and
/// the frame is never drawn. The wake makes the next iteration re-check the
/// flag.
pub(crate) fn request_frame() {
    NEEDS_REDRAW.store(true, Ordering::Relaxed);

    #[cfg(not_wasm)]
    {
        // On desktop about_to_wait runs after the frame, so a main thread
        // request_frame is picked up this iteration and a wake is redundant.
        // Waking from the main thread there instead livelocks the loop.
        #[cfg(not_ios)]
        if is_main_thread() {
            return;
        }

        wake_loop();
    }
}

#[cfg(not_wasm)]
fn wake_loop() {
    if let Some(proxy) = WAKE_PROXY.lock().as_ref()
        && proxy.send_event(UserEvent::Wake).is_err()
    {
        // The event loop already closed, the app is shutting down.
        log::trace!("wake requested after the event loop closed");
    }
}

/// What keeps the loop going between 2 frames of continuous work.
#[cfg(not_wasm)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pacer {
    /// The loop polls. Every iteration draws a frame, and the draw waits
    /// for the screen, so the loop does not spin.
    Poll,
    /// The loop sleeps, and every drawn frame wakes it once to ask for the
    /// next one. For iOS and tvOS. winit runs `Poll` there as a run loop
    /// timer that fires again at once, and `UIKit` draws once per screen
    /// refresh however often a redraw is asked. A polling loop so spins
    /// between 2 frames, which took a whole core of an iPhone.
    FrameWake,
}

#[cfg(not_wasm)]
impl Pacer {
    pub(crate) const fn of_platform() -> Self {
        if Platform::IOS {
            Self::FrameWake
        } else {
            Self::Poll
        }
    }

    /// Whether a frame that was just drawn has to wake the loop.
    fn wakes_after_frame(self, continuous: bool) -> bool {
        self == Self::FrameWake && continuous
    }
}

/// Call after every drawn frame. Under `Pacer::FrameWake` nothing else
/// brings the loop back to `about_to_wait` while continuous work lives,
/// also not work that began while this frame was drawn.
#[cfg(not_wasm)]
pub(crate) fn frame_drawn(continuous: bool) {
    if Pacer::of_platform().wakes_after_frame(continuous) {
        wake_loop();
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
/// level. The loop keeps drawing while this holds, by the `Pacer` of the
/// platform, then sleeps in Wait once it clears. A redraw requested from
/// `about_to_wait` does not re-arm a loop already asleep in Wait, so a
/// per-frame flag is not enough to keep continuous work drawing. The presence
/// of the work is.
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
pub(crate) fn frame_pacing(
    visibility: Visibility,
    continuous: bool,
    pending: bool,
    pacer: Pacer,
) -> Option<ControlFlow> {
    match visibility {
        Visibility::Covered => return None,
        Visibility::CoveredScreenshot => return Some(ControlFlow::Wait),
        Visibility::Visible => {}
    }
    if continuous {
        return Some(match pacer {
            Pacer::Poll => ControlFlow::Poll,
            Pacer::FrameWake => ControlFlow::Wait,
        });
    }
    pending.then_some(ControlFlow::Wait)
}

#[cfg(all(test, not_wasm))]
mod test {
    use winit::event_loop::ControlFlow;

    use super::{Pacer, Visibility, frame_pacing};

    const PACERS: [Pacer; 2] = [Pacer::Poll, Pacer::FrameWake];

    // Regression: a minimized window with a live animation used to keep the
    // loop polling at full speed, drawing frames nobody could see.
    #[test]
    fn covered_window_holds_continuous_work() {
        for pacer in PACERS {
            assert_eq!(frame_pacing(Visibility::Covered, true, true, pacer), None);
            assert_eq!(frame_pacing(Visibility::Covered, false, true, pacer), None);
            assert_eq!(frame_pacing(Visibility::Covered, false, false, pacer), None);
        }
    }

    // A screenshot of a covered window is drawn offscreen, so the request
    // must still get its one frame, and only one, no polling.
    #[test]
    fn covered_window_still_answers_a_screenshot() {
        for pacer in PACERS {
            assert_eq!(
                frame_pacing(Visibility::CoveredScreenshot, true, true, pacer),
                Some(ControlFlow::Wait)
            );
            assert_eq!(
                frame_pacing(Visibility::CoveredScreenshot, false, false, pacer),
                Some(ControlFlow::Wait)
            );
        }
    }

    #[test]
    fn visible_window_paces_as_before() {
        assert_eq!(
            frame_pacing(Visibility::Visible, true, false, Pacer::Poll),
            Some(ControlFlow::Poll)
        );
        assert_eq!(
            frame_pacing(Visibility::Visible, true, true, Pacer::Poll),
            Some(ControlFlow::Poll)
        );
        assert_eq!(
            frame_pacing(Visibility::Visible, false, true, Pacer::Poll),
            Some(ControlFlow::Wait)
        );
        assert_eq!(frame_pacing(Visibility::Visible, false, false, Pacer::Poll), None);
    }

    // Regression: an iPhone polled while an animation lived. Its main
    // thread took 96 percent of a core, the drawing 3.5 percent of it.
    #[test]
    fn frame_wake_never_polls() {
        for continuous in [true, false] {
            for pending in [true, false] {
                assert_ne!(
                    frame_pacing(Visibility::Visible, continuous, pending, Pacer::FrameWake),
                    Some(ControlFlow::Poll)
                );
            }
        }
        assert_eq!(
            frame_pacing(Visibility::Visible, true, false, Pacer::FrameWake),
            Some(ControlFlow::Wait)
        );
        assert_eq!(
            frame_pacing(Visibility::Visible, false, false, Pacer::FrameWake),
            None
        );
    }

    // A loop that sleeps draws the next frame of an animation only when
    // the frame before it wakes the loop. A polling loop needs no wake,
    // and a wake from the main thread livelocks a desktop loop.
    #[test]
    fn only_frame_wake_wakes_after_a_frame_of_continuous_work() {
        assert!(Pacer::FrameWake.wakes_after_frame(true));
        assert!(!Pacer::FrameWake.wakes_after_frame(false));
        assert!(!Pacer::Poll.wakes_after_frame(true));
        assert!(!Pacer::Poll.wakes_after_frame(false));
    }
}
