//! The app of the hot reload lane, `make hot-test`, see `docs/hot-reload.md`.
//!
//! The lane builds it 2 times as a dynamic library, the second time with the
//! `second` feature, which stands for a saved change. It then swaps the 2 in
//! a running app and reads what each one leaves: the color of the screen and
//! a marker file.

#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use std::fs::write;

use hilen::{
    App,
    filesystem::Paths,
    refs::{Own, Weak},
    ui::{Setup, UIManager, View, view},
};
use log::error;

#[cfg(not(feature = "second"))]
const GENERATION: &str = "first";
#[cfg(feature = "second")]
const GENERATION: &str = "second";

#[cfg(not(feature = "second"))]
const COLOR: &str = "#FF0000";
#[cfg(feature = "second")]
const COLOR: &str = "#00FF00";

/// The lane waits for this file to name the library it just put out.
const MARKER: &str = "hot-generation";

#[view]
struct Screen {}

impl Setup for Screen {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color(COLOR);

        if let Err(err) = write(Paths::storage().join(MARKER), GENERATION) {
            error!("the marker of the hot reload lane was not written: {err}");
        }
    }
}

#[derive(Default)]
pub struct HotTestApp;

impl App for HotTestApp {
    fn make_root_view(&self) -> Own<dyn View> {
        Screen::new()
    }
}

hilen::register_app!(HotTestApp);
