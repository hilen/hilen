use std::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
};

use log::info;

use crate::{
    app_starter::hilen_start_with_app,
    deps::refs::{Own, main_lock::MainLock},
    gm::flat::Size,
    system::{UpdateSource, app_update_source},
    ui::View,
    window::WindowPlacement,
};

pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;

/// The crates of the engine that log at debug level. Every other crate is
/// held to warnings, with the `log_targets` of the app let through too.
const ENGINE_LOG_TARGETS: &[&str] = &["hilen", "inspector", "netrun"];

/// True when a log line of `target` comes through at debug level: the crate
/// is one of the engine or of the app, or a module of one. `hilen_server`
/// is not `hilen`.
pub(crate) fn is_debug_target(target: &str, app_targets: &[&str]) -> bool {
    ENGINE_LOG_TARGETS.iter().chain(app_targets).any(|name| {
        target
            .strip_prefix(name)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
    })
}

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

static QUIT: AtomicBool = AtomicBool::new(false);

/// Calls `App::will_quit`, once, when the app ends. On the main thread.
pub(crate) fn quit() {
    let Some(app) = APP.get_mut().as_deref() else {
        return;
    };
    if QUIT.swap(true, Ordering::Relaxed) {
        return;
    }
    info!("The app quits");
    app.will_quit();
}

pub trait App {
    fn before_launch(&self) {}
    fn after_launch(&self) {}
    fn make_root_view(&self) -> Own<dyn View>;

    /// The size of a desktop window on its very first start and of the
    /// headless surface, in physical pixels. A 2x display opens the window
    /// at half as many points, and a display too small for it shrinks the
    /// window to fit. The default is 1200 by 900 points on a 2x display.
    /// Every later start opens the window where `window_placement` says.
    fn initial_size(&self) -> Size {
        (2400, 1800).into()
    }

    /// The desktop window placement to restore at launch instead of
    /// `initial_size`: the size, the place, the maximized state and the
    /// display. A placement whose monitor is no longer attached is
    /// centered on the primary display instead, see `resolve`.
    ///
    /// The default is the engine's own memory,
    /// `WindowPlacement::remembered`, the placement the last run left in
    /// the data folder of the app. So an app that writes neither this hook
    /// nor `window_placement_changed` opens where it was closed. An app
    /// that keeps the placement somewhere else writes both hooks. An app
    /// that wants the same window on every start returns `None` here and
    /// leaves the body of the other one empty.
    fn window_placement(&self) -> Option<WindowPlacement> {
        WindowPlacement::remembered()
    }

    /// Fires on every desktop window resize and move with the fresh
    /// placement, many times a second during a drag. It does not fire
    /// while the window is minimized, Windows reports a minimized window
    /// off screen with no size.
    ///
    /// The default hands it to the engine's own memory,
    /// `WindowPlacement::remember`, which writes it to disk a moment
    /// later on another thread, and once more when the app ends. An app
    /// that writes this hook saves by itself and the engine keeps nothing.
    fn window_placement_changed(&self, placement: &WindowPlacement) {
        placement.remember();
    }

    /// Runs once on the main thread right before the app ends, where the
    /// system says that it ends: a closed window, Cmd and Q, the end of
    /// the event loop, and `applicationWillTerminate` on iOS. iOS sends that
    /// only for an app that runs at the time, a suspended app that is
    /// closed from the app switcher or killed by the system gets no call.
    /// Android reports no end at all, so it never calls this. A killed
    /// process never calls it either. Work that must happen before the
    /// app goes away on a phone listens to `AppActivity::changed` too.
    fn will_quit(&self) {}

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

    /// The public half of the app's update key as hex, the one thing an
    /// app gives to update itself on the desktop. With it the engine
    /// checks for a new version after launch and keeps the result in
    /// `system::UpdateState`. `None` leaves self update off.
    fn update_key(&self) -> Option<&'static str> {
        None
    }

    /// The address of the update manifest, for an app that does not ship
    /// from the default download server. `None` means
    /// `<DEFAULT_UPDATE_HOST>/<project_name>/updater.json`.
    fn update_url(&self) -> Option<String> {
        None
    }

    /// Returns where `system::Updater` checks for new versions, `None`
    /// to disable self update, or a configuration error. The default
    /// builds it from `update_key` and `update_url`, an app overrides
    /// this only to decide the source at run time.
    fn update_source(&self) -> PinnedFuture<Option<UpdateSource>> {
        let source = app_update_source(self.update_key(), self.update_url());
        Box::pin(async move { Ok(source) })
    }
}

#[cfg(ios)]
unsafe extern "C" {
    #[allow(improper_ctypes_definitions)]
    #[allow(improper_ctypes)]
    pub(crate) fn hilen_create_app() -> Box<dyn App>;
}

// An iOS app always has `register_app!`, like `hilen_create_app` above. A
// hot build names its data folder with the project name, see
// `Paths::storage`, and a Sentry event names its release with both.
#[cfg(ios)]
unsafe extern "Rust" {
    pub(crate) safe fn hilen_project_name() -> &'static str;
    pub(crate) safe fn hilen_app_version() -> &'static str;
}

#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
#[allow(improper_ctypes_definitions)]
#[allow(improper_ctypes)]
pub extern "C" fn hilen_create_app() -> Box<dyn App> {
    panic!("you need to use hilen::register_app!(YourApp) macro")
}

/// When the code of the app was last compiled, in unix seconds, what
/// `hilen-inspect build-time` reads. `register_app!` stamps the final crate
/// of the app, and cargo compiles that crate again after any change below
/// it, the engine included. The stamp of the engine alone is older than a
/// rebuild of only the app, so it called a current app stale. An app with no
/// `register_app!` links this one and answers with the stamp of the engine.
#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_app_build_time() -> u64 {
    env!("HILEN_BUILD_TIME").parse().expect("HILEN_BUILD_TIME is not a number")
}

// An iOS app is a static library and always has `register_app!`, like
// `hilen_create_app` above.
#[cfg(ios)]
unsafe extern "Rust" {
    pub(crate) safe fn hilen_app_build_time() -> u64;
}

/// The `project_name` of the app's `hilen.toml`, the name of its data
/// folder and of its Sentry release. `register_app!` puts the real one into
/// the final crate, this one is linked when an app has no `register_app!`.
#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_project_name() -> &'static str {
    panic!("The app has no name, add hilen::register_app!(YourApp) to its main.rs")
}

/// The url scheme of the `[links]` table of the app's `hilen.toml`, which a
/// desktop start looks for in its arguments, see `system::Links`.
/// `register_app!` puts the real one into the final crate, an app without it
/// has no scheme.
#[cfg(desktop)]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_link_scheme() -> Option<&'static str> {
    None
}

/// The files an app packed into its binary with `embed_assets!`, each with
/// its name below the `assets` folder. An app without the macro links this
/// empty list.
#[cfg(desktop)]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_embedded_assets() -> &'static [(&'static str, &'static [u8])] {
    &[]
}

/// Packs the `fonts`, `images`, `models` and `sounds` folders of the app's
/// `assets` into the binary. Call it once, next to `register_app!`. A
/// desktop app that ships as 1 executable needs it, an installed app has no
/// `assets` folder. The engine reads a file from disk when it is there and
/// from the packed copy when it is not, so a run from the repo still sees
/// an edited file at once. Phones and the browser carry their assets in
/// their own bundle, there the macro adds nothing.
#[macro_export]
macro_rules! embed_assets {
    () => {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_embedded_assets() -> &'static [(&'static str, &'static [u8])] {
            hilen::embedded_assets!()
        }
    };
}

/// The version of the app, what the updater compares a manifest against and
/// what a Sentry release carries. `register_app!` puts the real one into the
/// final crate: the `version` of `hilen.toml` on iOS, tvOS and Android, whose
/// projects are stamped with it, the version of the package everywhere else.
#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_app_version() -> &'static str {
    panic!("The app has no version, add hilen::register_app!(YourApp) to its main.rs")
}

#[macro_export]
macro_rules! register_app {
    ($app:ty) => {
        pub use hilen;

        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_project_name() -> &'static str {
            hilen::project_name!()
        }

        // A phone and a TV app is stamped with the version of hilen.toml.
        // A desktop release, its updater and `make patch` use Cargo.toml.
        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_app_version() -> &'static str {
            if cfg!(any(
                target_os = "ios",
                target_os = "tvos",
                target_os = "android"
            )) {
                hilen::app_version!()
            } else {
                env!("CARGO_PKG_VERSION")
            }
        }

        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_app_build_time() -> u64 {
            hilen::build_time!()
        }

        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_link_scheme() -> Option<&'static str> {
            hilen::link_scheme!()
        }

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
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::{
        App,
        app::is_debug_target,
        deps::{hreads::set_current_thread_as_main, refs::Own},
        ui::{Container, Setup, View},
    };

    #[test]
    fn debug_targets_are_the_engine_and_the_app() {
        assert!(is_debug_target("hilen", &[]));
        assert!(is_debug_target("hilen::window::image", &[]));
        assert!(is_debug_target("flixen::net", &["flixen"]));
        assert!(!is_debug_target("hilen_server", &[]));
        assert!(!is_debug_target("naga::front::wgsl", &["flixen"]));
        assert!(!is_debug_target("wgpu_hal::gles", &[]));
    }

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

    /// This test binary has no `register_app!`, so the stub of the engine
    /// answers. An app in that state must stop with the fix in the message
    /// and never fall back to a folder named after its exe.
    #[cfg(desktop)]
    #[test]
    #[should_panic(expected = "add hilen::register_app!")]
    fn an_app_without_register_app_has_no_name() {
        crate::app::hilen_project_name();
    }

    static QUITS: AtomicU32 = AtomicU32::new(0);

    struct Quits;

    impl App for Quits {
        fn make_root_view(&self) -> Own<dyn View> {
            Container::new()
        }

        fn will_quit(&self) {
            QUITS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn the_app_hears_once_that_it_quits() {
        set_current_thread_as_main();
        crate::app::set_app(Box::new(Quits));

        crate::app::quit();
        crate::app::quit();

        assert_eq!(QUITS.load(Ordering::Relaxed), 1);
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
