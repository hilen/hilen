//! The search for running apps on the local network, for `hilen-inspect`.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use anyhow::Result;
use tokio::time::{Instant, timeout_at};

use super::SERVICE_TYPE;
use crate::discovery::{ServiceBrowser, ServiceEvent};

/// The longest a search takes.
const DEADLINE: Duration = Duration::from_secs(3);
/// After a hit the search waits this long for the next one, then ends.
const QUIET: Duration = Duration::from_millis(700);

/// Every running app with an inspect server, its id and its address. Cuts
/// the wait short when something is found: waits a little longer after each
/// hit to catch the others, then returns. A phone on a cable answers with
/// the address of the cable too, the address of the real network comes
/// first in a found service and wins.
pub async fn discover() -> Result<HashMap<String, SocketAddr>> {
    let mut browser = ServiceBrowser::start(SERVICE_TYPE)?;
    let mut apps = HashMap::new();

    let deadline = Instant::now() + DEADLINE;
    let mut cutoff = deadline;

    while let Ok(event) = timeout_at(deadline.min(cutoff), browser.next_event()).await {
        let ServiceEvent::Found(service) = event? else {
            continue;
        };
        let Some(app_id) = service.text.get("app_id") else {
            continue;
        };
        apps.insert(app_id.clone(), service.socket_address());
        cutoff = Instant::now() + QUIET;
    }

    Ok(apps)
}

#[cfg(all(test, apple))]
mod test {
    use std::net::SocketAddr;

    use super::discover;
    use crate::inspect::bonjour::announce;

    /// The announce of an app through the system and the search of a tool
    /// meet.
    #[tokio::test]
    async fn an_app_announced_through_bonjour_is_discovered() {
        let app_id = format!("T{}", fastrand::u32(100_000..1_000_000));
        announce(&app_id, "discovery-test", 45_123).unwrap();
        let apps = discover().await.unwrap();
        assert_eq!(apps.get(&app_id).map(SocketAddr::port), Some(45_123));
    }
}
