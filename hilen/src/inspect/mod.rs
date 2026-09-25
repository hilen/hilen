#![cfg(feature = "inspect")]

pub mod protocol;

mod edit_log;
mod hold;
#[cfg(any(desktop, wasm))]
mod hover;
mod inspect_service;
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
