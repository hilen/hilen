use std::pin::Pin;

use crate::{
    app_starter::hilen_start_with_app,
    deps::refs::{Own, main_lock::MainLock},
    gm::flat::Size,
    system::UpdateSource,
    ui::View,
    window::WindowPlacement,
};

pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;

/// The running app, reachable for as long as it runs.
///
/// It lives here rather than inside `AppRunner` because more than the runner
/// needs it. A UI test run tears the root view down, and putting it back means
/// asking the app for a new one long after launch.
static APP: MainLock<Option<Box<dyn App>>> = MainLock::new();

pub(crate) fn set_app(app: Box<dyn App>) {
    *APP.get_mut() = Some(app);
}

pub(crate) fn app() -> &'static dyn App {
    APP.get_mut()
        .as_deref()
        .expect("App is not set. `hilen_start_with_app` does that.")
}

pub trait App {
    fn before_launch(&self) {}
    fn after_launch(&self) {}
    fn make_root_view(&self) -> Own<dyn View>;

    /// The size of a fresh desktop window and of the headless surface, in
    /// physical pixels. A 2x display opens the window at half as many
    /// points, and a display too small for it shrinks the window to fit.
    /// The default is 1200 by 900 points on a 2x display.
    fn initial_size(&self) -> Size {
        (2400, 1800).into()
    }

    /// A saved desktop window placement to restore at launch instead of
    /// `initial_size`. A placement whose monitor is no longer attached is
    /// centered on the primary display instead, see `resolve`.
    fn window_placement(&self) -> Option<WindowPlacement> {
        None
    }

    /// Fires on every desktop window resize and move with the fresh
    /// placement. This is the place to save it, there is no close hook
    /// because Cmd+Q on macOS ends the process without one.
    /// It does not fire while the window is minimized, Windows reports a
    /// minimized window off screen with no size.
    fn window_placement_changed(&self, _placement: &WindowPlacement) {}

    /// Log targets of the app itself, usually just the crate name. The
    /// engine logger silences everything except its own crates to warnings,
    /// targets listed here come through at debug level like the engine's.
    fn log_targets(&self) -> &'static [&'static str] {
        &[]
    }

    /// How many log files of the app stay on disk, the one of this launch
    /// counted in. Older ones are removed at every start, before the new
    /// file is written. The default is 10. This launch always writes its
    /// file, so 0 keeps 1 like 1 does. Android and the browser write no
    /// log file and never read this.
    fn log_files_kept(&self) -> usize {
        10
    }

    fn start()
    where Self: Default + Sized + 'static {
        hilen_start_with_app(Box::new(Self::default()));
    }

    /// Returns a Sentry DSN, `None` to disable Sentry, or a configuration
    /// error.
    fn sentry_url(&self) -> PinnedFuture<Option<String>> {
        Box::pin(async { Ok(None) })
    }

    /// Returns where `system::Updater` checks for new versions, `None`
    /// to disable self update, or a configuration error.
    fn update_source(&self) -> PinnedFuture<Option<UpdateSource>> {
        Box::pin(async { Ok(None) })
    }
}

#[cfg(ios)]
unsafe extern "C" {
    #[allow(improper_ctypes_definitions)]
    #[allow(improper_ctypes)]
    pub(crate) fn hilen_create_app() -> Box<dyn App>;
}

#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
#[allow(improper_ctypes_definitions)]
#[allow(improper_ctypes)]
pub extern "C" fn hilen_create_app() -> Box<dyn App> {
    panic!("you need to use hilen::register_app!(YourApp) macro")
}

#[macro_export]
macro_rules! register_app {
    ($app:ty) => {
        pub use hilen;

        #[unsafe(no_mangle)]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn hilen_create_app() -> Box<dyn hilen::App> {
            use hilen::App;

            fn check_trait<T: hilen::App>() {}
            check_trait::<$app>();

            Box::new(<$app>::default())
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        App,
        deps::refs::Own,
        ui::{Container, Setup, View},
    };

    struct Plain;

    impl App for Plain {
        fn make_root_view(&self) -> Own<dyn View> {
            Container::new()
        }
    }

    struct FewLogs;

    impl App for FewLogs {
        fn make_root_view(&self) -> Own<dyn View> {
            Container::new()
        }

        fn log_files_kept(&self) -> usize {
            5
        }
    }

    #[test]
    fn an_app_keeps_10_log_files_unless_it_names_another_count() {
        // Through the box, the way the starter reads the hook.
        let plain: Box<dyn App> = Box::new(Plain);
        let few: Box<dyn App> = Box::new(FewLogs);

        assert_eq!(plain.log_files_kept(), 10);
        assert_eq!(few.log_files_kept(), 5);
    }
}
