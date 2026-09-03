#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

mod api;
mod app;
mod interface;
mod levels;
mod no_physics;
mod scenes;

// The library build is what iOS and ui-test link. Exposing the app entry
// keeps its whole chain reachable, so it is not dead code off the binary.
pub use app::DemoApp;
#[cfg(not(ios))]
pub use hilen;

#[cfg(ios)]
hilen::register_app!(DemoApp);
