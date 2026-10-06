use std::pin::Pin;

use crate::{
    app_starter::hilen_start_with_app,
    deps::refs::{Own, main_lock::MainLock},
    gm::flat::Size,
    system::{UpdateSource, app_update_source},
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

#[cfg(not(ios))]
#[unsafe(no_mangle)]
#[linkage = "weak"]
#[allow(improper_ctypes_definitions)]
#[allow(improper_ctypes)]
pub extern "C" fn hilen_create_app() -> Box<dyn App> {
    panic!("you need to use hilen::register_app!(YourApp) macro")
}

/// The `project_name` of the app's `hilen.toml`, the name of its data
/// folder. `register_app!` puts the real one into the final crate, this
/// one is linked when an app has no `register_app!`.
#[cfg(desktop)]
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "Rust" fn hilen_project_name() -> &'static str {
    panic!("The app has no name, add hilen::register_app!(YourApp) to its main.rs")
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

/// The version of the app's own package, what the updater compares a
/// manifest against. `register_app!` puts the real one into the final crate.
#[cfg(desktop)]
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

        #[unsafe(no_mangle)]
        pub extern "Rust" fn hilen_app_version() -> &'static str {
            env!("CARGO_PKG_VERSION")
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

    /// This test binary has no `register_app!`, so the stub of the engine
    /// answers. An app in that state must stop with the fix in the message
    /// and never fall back to a folder named after its exe.
    #[cfg(desktop)]
    #[test]
    #[should_panic(expected = "add hilen::register_app!")]
    fn an_app_without_register_app_has_no_name() {
        crate::app::hilen_project_name();
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
