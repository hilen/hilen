#![cfg(feature = "inspect")]

pub mod protocol;

mod edit_log;
mod hold;
#[cfg(any(desktop, wasm))]
mod hover;
mod inspect_service;
#[cfg(test)]
mod release_guard;
mod view_conversion;
mod wait;
#[cfg(wasm)]
pub(crate) mod web_transport;

pub mod views;

pub use self::{
    protocol::{AppCommand, InspectorCommand, ui::ViewRepr},
    view_conversion::{ViewToInspect, weak_to_id},
};
pub use crate::inspect::inspect_service::InspectService;

/// Every build with this module carries these bytes, the start functions keep
/// them alive. The release scripts in `build/shared` and `web_mount` in
/// `hilen-server` search a built file for them and refuse to ship it. Those
/// copies must stay the same as this one.
pub(crate) const MARKER: &str = "hilen-inspect-server-compiled-in";
