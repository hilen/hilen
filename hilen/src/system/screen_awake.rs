use super::AppActivity;

#[cfg(feature = "ui-tests")]
#[path = "screen_awake_test.rs"]
mod tests_ui;
use crate::deps::{
    hreads::{assert_main_thread, on_main},
    refs::main_lock::MainLock,
};

static STATE: MainLock<AwakeState> = MainLock::new();

/// Keeps the mobile display awake while the app is active. Does not prevent
/// manual locking or enable background execution. Other targets retain their
/// normal display policy. Acquire on the main thread; dropping releases it.
/// Multiple owners may hold a guard without cancelling each other's request.
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

    #[cfg(not(any(ios, android)))]
    log::trace!("Mobile screen awake request: {enabled}");
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
