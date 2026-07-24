#![allow(incomplete_features)]
#![allow(clippy::single_component_path_imports)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]
#![cfg_attr(not(ios), feature(linkage))]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![feature(generic_const_exprs)]
#![feature(const_type_name)]

// The #[view] and #[level] macros emit `test_engine::` paths. This alias
// makes them resolve inside the crate itself.
extern crate self as test_engine;

mod app_runner;
mod assets;
mod assets_paths;
mod level_drawer;
mod web;

mod app;
mod app_starter;
mod config;
mod dispatch_tools;
mod game_drawer;
#[cfg(target_os = "ios")]
mod ios_log;
mod pipelines;

pub mod audio;
pub mod filesystem;
pub mod game;
pub mod generate;
pub mod gm;
pub mod inspect;
pub mod level;
pub mod render;
pub mod store;
pub mod ui;
pub mod window;

pub use app::*;
pub use app_starter::*;
pub use educe;

pub use crate::ui::{launch_app, ui_test};

pub mod refs {

    pub mod manage {
        pub use refs::manage::*;
    }

    pub use refs::{__internal_deps, AsAny, Own, Weak, managed, vec::OwnVec, weak_from_ref};
}

pub mod reflected {
    pub use ::reflected::{Field, Reflected, ToReflectedString, ToReflectedVal, Type};
}

pub mod time {
    pub use web_time::*;
}

pub use app_runner::AppRunner;
pub use vents::{Event, OnceEvent};

pub use crate::window::{RenderPass, VertexBuffer, Window, cast_slice, image::ToImage};

pub mod net {
    pub use netrun::rest::*;
    #[cfg(not_wasm)]
    pub use netrun::secret::*;
}

pub mod dispatch {
    #[cfg(not_wasm)]
    pub use ::hreads::first_ok;
    pub use ::hreads::{
        after, from_main, is_main_thread, ok_main, on_main, sleep, spawn, wait_async, wait_for_next_frame,
    };

    pub use crate::{dispatch_tools::*, gm::drop_on_main};
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
