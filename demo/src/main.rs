#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use crate::app::DemoApp;

mod api;
mod app;
mod interface;
mod levels;
mod no_physics;

hilen::register_app!(DemoApp);

fn main() {
    hilen::hilen_start_app();
}
