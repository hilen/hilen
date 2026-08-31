#![allow(incomplete_features)]
#![allow(internal_features)]
#![allow(clippy::single_component_path_imports)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]
#![cfg_attr(not(ios), feature(linkage))]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![feature(generic_const_exprs)]
#![feature(const_type_name)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]

// The #[view] and #[level] macros emit `hilen::` paths. This alias
// makes them resolve inside the crate itself.
extern crate self as hilen;

mod deps;

mod app_runner;
mod assets;
mod assets_paths;
#[cfg(feature = "level")]
mod level_drawer;
mod web;

mod app;
mod app_starter;
mod config;
mod dispatch_tools;
#[cfg(feature = "level")]
mod game_drawer;
#[cfg(target_os = "ios")]
mod ios_log;
#[cfg(not_wasm)]
mod log_file;
mod pipelines;

#[cfg(feature = "audio")]
pub mod audio;
pub mod bug_report;
pub mod filesystem;
#[cfg(feature = "level")]
pub mod game;
pub mod generate;
pub mod gm;
pub mod inspect;
#[cfg(feature = "level")]
pub mod level;
pub mod render;
pub mod store;
pub mod system;
pub mod ui;
pub mod window;

pub use app::*;
pub use app_starter::*;
pub use educe;

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
}

pub use app_runner::AppRunner;
pub use assets::Assets;
pub use bug_report::BugReport;
#[cfg(not_wasm)]
pub use log_file::{log_dir, log_file_path};

pub use crate::{
    deps::vents::{Event, OnceEvent},
    window::{RenderPass, VertexBuffer, Window, cast_slice, image::ToImage},
};

pub mod net {
    #[cfg(not_wasm)]
    pub use crate::deps::netrun::secret::*;
    pub use crate::deps::netrun::{Function, System, local_ip, rest::*, ws::*};
}

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

#[cfg(target_os = "android")]
pub type AndroidApp = winit::platform::android::activity::AndroidApp;

/// Every UI test, from the engine, the corpus and the app. One map, filled by
/// a ctor per view before `main`, so the count is known without running
/// anything and nothing has to merge lists.
pub static UI_TESTS: __internal_macro_deps::Mutex<
    std::collections::BTreeMap<String, crate::ui_test::UITestEntry>,
> = __internal_macro_deps::Mutex::new(std::collections::BTreeMap::new());

/// Every level test, the same shape as `UI_TESTS`, filled by a ctor per
/// level.
#[cfg(feature = "level")]
pub static LEVEL_TESTS: __internal_macro_deps::Mutex<
    std::collections::BTreeMap<String, crate::ui_test::UITestEntry>,
> = __internal_macro_deps::Mutex::new(std::collections::BTreeMap::new());
