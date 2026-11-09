#![allow(incomplete_features)]
#![allow(internal_features)]
#![allow(clippy::single_component_path_imports)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]
#![cfg_attr(not(ios), feature(linkage))]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![recursion_limit = "256"]
#![feature(const_type_name)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]

// The #[view] and #[level] macros emit `hilen::` paths. This alias
// makes them resolve inside the crate itself.
extern crate self as hilen;

mod deps;

#[cfg(feature = "gpu")]
mod app_runner;
mod assets;
#[cfg(feature = "gpu")]
mod assets_paths;
#[cfg(all(desktop, feature = "gpu"))]
mod assets_root;
#[cfg(desktop)]
mod embedded;
#[cfg(feature = "level")]
mod level_drawer;
#[cfg(feature = "scene")]
mod scene_drawer;
mod web;
mod web_log;
mod web_visit;

mod app;
#[cfg(feature = "gpu")]
mod app_starter;
mod config;
mod dispatch_tools;
mod frame_time;
#[cfg(feature = "level")]
mod game_drawer;
#[cfg(hot)]
mod hot;
#[cfg(target_os = "ios")]
mod ios_log;
#[cfg(not_wasm)]
mod log_file;
mod monotonic;
#[cfg(all(any(desktop, target_os = "android"), feature = "gpu"))]
mod panic_log;
#[cfg(feature = "gpu")]
mod pipelines;
mod wipe;

pub mod app_commands;
#[cfg(sound)]
pub mod audio;
#[cfg(feature = "video")]
pub mod video;
#[cfg(all(feature = "video", not(any(ffmpeg, wasm))))]
compile_error!("the video feature is for desktop, iOS, tvOS and the browser for now, see docs/video.md");
#[cfg(feature = "biometric")]
pub mod biometric;
#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(feature = "bug-report")]
pub mod bug_report;
#[cfg(feature = "camera")]
pub mod camera;
#[cfg(feature = "discovery")]
pub mod discovery;
pub mod filesystem;
#[cfg(feature = "level")]
pub mod game;
#[cfg(feature = "noise")]
pub mod generate;
#[cfg(feature = "global-keys")]
pub mod global_keys;
pub mod gm;
#[cfg(feature = "google-access")]
pub mod google_access;
pub mod image;
pub mod inspect;
#[cfg(feature = "level")]
pub mod level;
#[cfg(feature = "login")]
pub mod login;
#[cfg(feature = "midi")]
pub mod midi;
#[cfg(feature = "push")]
pub mod push;
#[cfg(feature = "gpu")]
pub mod render;
#[cfg(feature = "scene")]
pub mod scene;
#[cfg(feature = "screen-capture")]
pub mod screen_capture;
#[cfg(any(feature = "login", feature = "google-access"))]
mod sign_in_button;
#[cfg(feature = "speech")]
pub mod speech;
pub mod store;
pub mod system;
pub mod text;
pub mod ui;
pub mod window;

pub use app::*;
#[cfg(feature = "gpu")]
pub use app_starter::*;
pub use educe;
/// The data of a file diff that `ui::DiffPanel` draws.
#[cfg(feature = "diff-view")]
pub use filediff;
/// The self update calls with no UI, for a binary with no window.
/// `system::Updater` and `system::UpdateState` sit on top of it.
#[cfg(feature = "updater")]
pub use hilen_updater as updater;

#[cfg(feature = "gpu")]
pub use crate::ui::{launch_app, ui_test};

pub mod refs {

    pub mod manage {
        pub use crate::deps::refs::manage::*;
    }

    pub use crate::{
        deps::refs::{__internal_deps, AsAny, Own, Weak, hreads, main_lock, vec::OwnVec, weak_from_ref},
        managed,
    };
}

pub mod reflected {
    pub use ::reflected::{Field, Reflected, ToReflectedString, ToReflectedVal, Type};
}

pub mod time {
    pub use web_time::*;

    pub use crate::{frame_time::frame_seconds, monotonic::monotonic_seconds};
}

#[cfg(feature = "gpu")]
pub use app_runner::AppRunner;
pub use assets::Assets;
#[cfg(feature = "bug-report")]
pub use bug_report::{BugReport, BugReportAttachment, BugReportScreenshot, BugReportStyle};
#[cfg(not_wasm)]
pub use log_file::{log_dir, log_file_path};
pub use project_proc::{app_version, build_time, embedded_assets, link_scheme, project_name};

#[cfg(feature = "gpu")]
pub use crate::window::{RenderPass, VertexBuffer, Window, cast_slice};
pub use crate::{
    deps::vents::{Event, OnceEvent},
    image::ToImage,
};

pub mod net {
    #[cfg(all(not_wasm, feature = "net"))]
    pub use crate::deps::netrun::pinned::pinned_client;
    #[cfg(all(not_wasm, feature = "infisical"))]
    pub use crate::deps::netrun::secret::*;
    pub use crate::deps::netrun::{Function, System};
    #[cfg(feature = "net")]
    pub use crate::deps::netrun::{local_ip, rest::*, transport::*, ws::*};
}

#[cfg(all(wasm, not(feature = "net")))]
compile_error!("a browser build loads its assets over the network, it needs the `net` feature of hilen");

#[cfg(all(not(feature = "gpu"), not(desktop)))]
compile_error!(
    "a build with no `gpu` feature is for desktop only, where a crate of shared views checks on its own"
);

pub mod dispatch {
    #[cfg(not_wasm)]
    pub use crate::deps::hreads::{first_ok, log_spawn};
    pub use crate::{
        deps::hreads::{
            after, from_main, is_main_thread, ok_main, on_main, sleep, spawn, wait_async, wait_for_next_frame,
        },
        dispatch_tools::*,
        gm::drop_on_main,
    };
}

pub mod __internal_macro_deps {
    pub use ctor;
    pub use parking_lot::Mutex;
}

pub use plat::Platform;

#[cfg(all(target_os = "android", feature = "gpu"))]
pub type AndroidApp = winit::platform::android::activity::AndroidApp;

/// Every UI test, from the engine, the suite and the app. One map, filled by
/// a ctor per view before `main`, so the count is known without running
/// anything and nothing has to merge lists.
#[cfg(feature = "gpu")]
pub static UI_TESTS: __internal_macro_deps::Mutex<
    std::collections::BTreeMap<String, crate::ui_test::UITestEntry>,
> = __internal_macro_deps::Mutex::new(std::collections::BTreeMap::new());

/// Every level test, the same shape as `UI_TESTS`, filled by a ctor per
/// level.
#[cfg(feature = "level")]
pub static LEVEL_TESTS: __internal_macro_deps::Mutex<
    std::collections::BTreeMap<String, crate::ui_test::UITestEntry>,
> = __internal_macro_deps::Mutex::new(std::collections::BTreeMap::new());

/// Every scene test, the same shape as `UI_TESTS`, filled by a ctor per
/// scene.
#[cfg(feature = "scene")]
pub static SCENE_TESTS: __internal_macro_deps::Mutex<
    std::collections::BTreeMap<String, crate::ui_test::UITestEntry>,
> = __internal_macro_deps::Mutex::new(std::collections::BTreeMap::new());
