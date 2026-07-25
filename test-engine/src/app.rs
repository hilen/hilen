use std::pin::Pin;

use refs::{Own, main_lock::MainLock};

use crate::{app_starter::test_engine_start_with_app, gm::flat::Size, ui::View};

pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>>>>;

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
        .expect("App is not set. `test_engine_start_with_app` does that.")
}

pub trait App {
    fn before_launch(&self) {}
    fn after_launch(&self) {}
    fn make_root_view(&self) -> Own<dyn View>;
    fn initial_size(&self) -> Size {
        (1200, 1000).into()
    }

    /// Log targets of the app itself, usually just the crate name. The
    /// engine logger silences everything except its own crates to warnings,
    /// targets listed here come through at debug level like the engine's.
    fn log_targets(&self) -> &'static [&'static str] {
        &[]
    }

    fn start()
    where Self: Default + Sized + 'static {
        test_engine_start_with_app(Box::new(Self::default()));
    }

    /// Returns a Sentry DSN, `None` to disable Sentry, or a configuration
    /// error.
    fn sentry_url(&self) -> PinnedFuture<Option<String>> {
        Box::pin(async { Ok(None) })
    }
}

#[cfg(ios)]
unsafe extern "C" {
    #[allow(improper_ctypes_definitions)]
    #[allow(improper_ctypes)]
    pub(crate) fn test_engine_create_app() -> Box<dyn App>;
}

#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
#[allow(improper_ctypes_definitions)]
#[allow(improper_ctypes)]
pub extern "C" fn test_engine_create_app() -> Box<dyn App> {
    panic!("you need to use test_engine::register_app!(YourApp) macro")
}

#[macro_export]
macro_rules! register_app {
    ($app:ty) => {
        pub use test_engine;

        #[unsafe(no_mangle)]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn test_engine_create_app() -> Box<dyn test_engine::App> {
            use test_engine::App;

            fn check_trait<T: test_engine::App>() {}
            check_trait::<$app>();

            Box::new(<$app>::default())
        }
    };
}
