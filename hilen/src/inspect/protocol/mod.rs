mod app_command;
#[cfg(macos)]
mod bonjour_browse;
#[cfg(not_wasm)]
mod discovery;
mod inspector_command;
#[cfg(not_wasm)]
mod transport;
pub mod ui;

pub use self::{app_command::*, inspector_command::*};
#[cfg(not_wasm)]
pub use self::{discovery::discover, transport::*};

pub const SERVICE_TYPE: &str = "_hilen-inspect._tcp.local.";
