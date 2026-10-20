//! The announce of an inspect server for an app that is not this process,
//! for a tool that serves the app in its place, like the page bridge of
//! `hilen-inspect`. The app is on the network as long as the value lives.

use anyhow::Result;
#[cfg(not(apple))]
use mdns_sd::{ServiceDaemon, ServiceInfo};

#[cfg(not(apple))]
use super::SERVICE_TYPE;
#[cfg(apple)]
use crate::inspect::bonjour::{Held, announce_held};

pub struct Announcement {
    _held: Held,
}

/// Announces the app `app_id` with the inspect server at `port` of this
/// machine, the same way an app announces its own.
pub fn announce(app_id: &str, name: &str, port: u16) -> Result<Announcement> {
    Ok(Announcement {
        _held: announce_held(app_id, name, port)?,
    })
}

/// The daemon answers for the app as long as it lives.
#[cfg(not(apple))]
type Held = ServiceDaemon;

#[cfg(not(apple))]
fn announce_held(app_id: &str, name: &str, port: u16) -> Result<Held> {
    let daemon = ServiceDaemon::new()?;
    daemon.register(
        ServiceInfo::new(
            SERVICE_TYPE,
            app_id,
            &format!("{app_id}.local."),
            "",
            port,
            &[("app_id", app_id), ("name", name)][..],
        )?
        .enable_addr_auto(),
    )?;
    Ok(daemon)
}
