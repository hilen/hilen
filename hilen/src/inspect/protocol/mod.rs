mod app_command;
mod inspector_command;
#[cfg(not_wasm)]
mod transport;
pub mod ui;

#[cfg(not_wasm)]
pub use self::transport::*;
pub use self::{app_command::*, inspector_command::*};

pub const SERVICE_TYPE: &str = "_hilen-inspect._tcp.local.";
