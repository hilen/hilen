use super::AppActivity;

#[cfg(all(feature = "ui-tests", desktop))]
#[path = "screen_awake_desktop_test.rs"]
mod tests_desktop;
#[cfg(feature = "ui-tests")]
#[path = "screen_awake_test.rs"]
mod tests_ui;
use crate::deps::{
    hreads::{assert_main_thread, on_main},
    refs::main_lock::MainLock,
};

static STATE: MainLock<AwakeState> = MainLock::new();

/// What the system shows as the reason the display stays on.
#[cfg(desktop)]
const REASON: &str = "hilen app keeps the screen on";

/// Keeps the display awake while the app is active, on a phone and on a
/// desktop. Does not prevent manual locking or enable background execution.
/// A browser keeps its normal display policy. Acquire on the main thread;
/// dropping releases it. Multiple owners may hold a guard without cancelling
/// each other's request.
#[must_use = "the screen stays awake only while the guard is held"]
pub struct ScreenAwake {
    state: &'static MainLock<AwakeState>,
}

impl ScreenAwake {
    pub fn acquire() -> Self {
        assert_main_thread();
        let state = STATE.get_mut();
        state.active = AppActivity::is_active();
        state.requests += 1;
        state.apply();
        Self { state: &STATE }
    }
}

impl Drop for ScreenAwake {
    fn drop(&mut self) {
        let state = self.state;
        on_main(move || {
            let state = state.get_mut();
            state.requests -= 1;
            state.apply();
        });
    }
}

pub(crate) fn set_active(active: bool) {
    let state = STATE.get_mut();
    state.active = active;
    state.apply();
}

#[derive(Default)]
struct AwakeState {
    requests: usize,
    active:   bool,
    applied:  bool,
}

impl AwakeState {
    fn requested(&self) -> bool {
        self.active && self.requests > 0
    }

    fn apply(&mut self) {
        let requested = self.requested();
        if self.applied != requested {
            set_screen_awake(requested);
            self.applied = requested;
        }
    }
}

fn set_screen_awake(enabled: bool) {
    #[cfg(ios)]
    {
        use objc2_foundation::MainThreadMarker;
        use objc2_ui_kit::UIApplication;

        let main = MainThreadMarker::new().expect("screen awake requires the main thread");
        UIApplication::sharedApplication(main).setIdleTimerDisabled(enabled);
    }

    #[cfg(android)]
    {
        use winit::platform::android::activity::WindowManagerFlags;

        use crate::app_starter::ANDROID_APP;

        let app = ANDROID_APP.lock();
        let app = app.as_ref().expect("AndroidApp is not set");
        let flag = WindowManagerFlags::KEEP_SCREEN_ON;
        let empty = WindowManagerFlags::empty();
        // android-activity marshals this onto Android's UI thread.
        app.set_window_flags(
            if enabled { flag } else { empty },
            if enabled { empty } else { flag },
        );
    }

    #[cfg(desktop)]
    {
        use keepawake::{Builder, KeepAwake};
        use log::warn;

        use crate::log_file::app_name;

        /// The hold the system has on file for this app, dropped to end it.
        static HOLD: MainLock<Option<KeepAwake>> = MainLock::new();

        *HOLD.get_mut() = if enabled {
            match Builder::default()
                .display(true)
                .reason(REASON)
                .app_name(app_name().unwrap_or_else(|_| "hilen".to_string()))
                .app_reverse_domain("io.hilen.app")
                .create()
            {
                Ok(hold) => Some(hold),
                Err(err) => {
                    warn!("the screen cannot be kept awake: {err}");
                    None
                }
            }
        } else {
            None
        };
    }

    #[cfg(wasm)]
    log::trace!("Screen awake request ignored in a browser: {enabled}");
}

#[cfg(test)]
mod tests {
    use super::AwakeState;

    #[test]
    fn multiple_owners_and_background_release() {
        let mut state = AwakeState {
            active: true,
            ..AwakeState::default()
        };
        assert!(!state.requested());
        state.requests = 2;
        assert!(state.requested());
        state.requests -= 1;
        assert!(state.requested());
        state.active = false;
        assert!(!state.requested());
        state.requests -= 1;
        state.active = true;
        assert!(!state.requested());
    }
}
