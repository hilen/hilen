pub mod basic;
/// Not in the browser lane: the page stopped answering in this test after
/// the camera opened, about 1 run in 3 on 2026-10-17, also without any other
/// change. The cause is not found.
#[cfg(not_wasm)]
mod camera;
pub mod code_editor;
pub mod complex;
pub mod containers;
pub mod file_browser;
pub mod helpers;
pub mod images;
pub mod input;
pub mod layout;
pub mod text;
#[cfg(feature = "video")]
pub mod video;
