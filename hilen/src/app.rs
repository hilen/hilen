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
    fn initial_size(&self) -> Size {
        (1200, 1000).into()
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
    fn window_placement_changed(&self, _placement: &WindowPlacement) {}

    /// Log targets of the app itself, usually just the crate name. The
    /// engine logger silences everything except its own crates to warnings,
    /// targets listed here come through at debug level like the engine's.
    fn log_targets(&self) -> &'static [&'static str] {
        &[]
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
