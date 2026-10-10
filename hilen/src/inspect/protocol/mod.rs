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

/// The port the inspect server of the first app on a device takes. A tool
/// that can not search the network, like one that reaches a phone over a
/// VPN, finds the app by the address of the device and this port. Every
/// other app on the device takes a free port.
pub const FIXED_PORT: u16 = 7435;
