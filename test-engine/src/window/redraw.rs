use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(all(not_wasm, not(target_os = "ios")))]
use hreads::is_main_thread;
#[cfg(not_wasm)]
use parking_lot::Mutex;
#[cfg(not_wasm)]
use winit::event_loop::EventLoopProxy;

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
pub(crate) fn continuous_render_active() -> bool {
    crate::ui::UIManager::has_live_animations() || !crate::level::LevelManager::no_level()
}
