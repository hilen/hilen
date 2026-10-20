#[cfg(not_wasm)]
mod announcement;
mod app_command;
#[cfg(not_wasm)]
mod discovery;
mod inspector_command;
#[cfg(not_wasm)]
mod transport;
pub mod ui;

#[cfg(not_wasm)]
pub use self::{
    announcement::{Announcement, announce},
    discovery::discover,
    transport::*,
};
pub use self::{app_command::*, inspector_command::*};

pub const SERVICE_TYPE: &str = "_hilen-inspect._tcp.local.";

/// The port the inspect server of the first app on a device takes. A tool
/// that can not search the network, like one that reaches a phone over a
/// VPN, finds the app by the address of the device and this port. Every
/// other app on the device takes a free port.
pub const FIXED_PORT: u16 = 7435;

/// The port a dev package of a TV app dials on the TV and `hilen-inspect
/// web-bridge` takes it on, carried between the two by an ssh tunnel. See
/// "An installed TV app" in `docs/inspect.md`.
pub const PAGE_PORT: u16 = 7436;
