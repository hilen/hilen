//! The search for running apps on the local network, for `hilen-inspect`.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use anyhow::Result;
use tokio::time::Instant;

/// The longest a search takes.
const DEADLINE: Duration = Duration::from_secs(3);
/// After a hit the search waits this long for the next one, then ends.
const QUIET: Duration = Duration::from_millis(700);

/// Every running app with an inspect server, its id and its address. Cuts
/// the wait short when something is found: waits a little longer after each
/// hit to catch the others, then returns.
#[cfg(macos)]
pub async fn discover() -> Result<HashMap<String, SocketAddr>> {
    use tokio::time::sleep;

    use super::bonjour_browse::{finish, found, start};

    const POLL: Duration = Duration::from_millis(50);

    start()?;

    let deadline = Instant::now() + DEADLINE;
    let mut cutoff = deadline;
    let mut count = 0;

    while Instant::now() < deadline.min(cutoff) {
        sleep(POLL).await;
        let now = found().len();
        if now > count {
            count = now;
            cutoff = Instant::now() + QUIET;
        }
    }

    Ok(finish())
}

#[cfg(not(macos))]
pub async fn discover() -> Result<HashMap<String, SocketAddr>> {
    use std::net::IpAddr;

    use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};
    use tokio::time::timeout_at;

    use super::SERVICE_TYPE;

    let mdns = ServiceDaemon::new()?;
    let events = mdns.browse(SERVICE_TYPE)?;

    let mut apps = HashMap::new();

    let deadline = Instant::now() + DEADLINE;
    let mut cutoff = deadline;

    loop {
        let until = deadline.min(cutoff);

        let Ok(Ok(event)) = timeout_at(until, events.recv_async()).await else {
            break;
        };

        let ServiceEvent::ServiceResolved(service) = event else {
            continue;
        };

        let Some(app_id) = service.txt_properties.get_property_val_str("app_id") else {
            continue;
        };

        let ip = service
            .addresses
            .iter()
            .map(ScopedIp::to_ip_addr)
            .find(IpAddr::is_ipv4)
            .or_else(|| service.addresses.iter().next().map(ScopedIp::to_ip_addr));

        let Some(ip) = ip else {
            continue;
        };

        apps.insert(app_id.to_string(), SocketAddr::new(ip, service.port));
        cutoff = Instant::now() + QUIET;
    }

    Ok(apps)
}
