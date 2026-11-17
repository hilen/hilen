#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use hilen::App;

use crate::app::InspectorApp;

mod app;
mod ui;

hilen::register_app!(InspectorApp);

fn main() {
    InspectorApp::start();
}
