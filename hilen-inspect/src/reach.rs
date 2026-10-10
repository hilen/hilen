//! How the tool reaches an app: a search of the local network, the address
//! the last search cached, or an address given with `--addr`. A VPN like
//! Tailscale carries no search, there only the address works.

use std::{
    collections::HashMap,
    env::{temp_dir, var},
    fs::{read_to_string, write},
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use hilen::inspect::protocol::{Client, FIXED_PORT, discover};
use serde_json::{from_str, to_string};
use tokio::{net::lookup_host, time::timeout};

pub(super) const NO_APPS: &str = "No running apps discovered. The app must be built with the `inspect` feature and running on the same network.";

/// Gives the address when `--addr` is not passed.
const ADDR_ENV: &str = "HILEN_INSPECT_ADDR";
/// The id of the app at a given address in a list of apps.
const GIVEN: &str = "given";
const CONNECT: Duration = Duration::from_secs(5);

/// The address to reach the app at, from `--addr` or from the variable,
/// none when neither is set. A host with no port means the fixed port the
/// first app on a device takes.
pub(super) async fn given(addr: Option<String>) -> Result<Option<SocketAddr>> {
    let Some(text) = addr.or_else(|| var(ADDR_ENV).ok()) else {
        return Ok(None);
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let found: Vec<SocketAddr> = if has_port(text) {
        lookup_host(text).await.map(Iterator::collect)
    } else {
        lookup_host((text, FIXED_PORT)).await.map(Iterator::collect)
    }
    .with_context(|| format!("no address for {text}"))?;
    match found.first() {
        Some(addr) => Ok(Some(*addr)),
        None => bail!("no address for {text}"),
    }
}

/// An IPv6 address has colons and no port, so only a last part that is a
/// number after a host with no colon counts.
fn has_port(text: &str) -> bool {
    text.parse::<SocketAddr>().is_ok()
        || text
            .rsplit_once(':')
            .is_some_and(|(host, port)| !host.contains(':') && port.parse::<u16>().is_ok())
}

/// The apps to pick from: the one at the given address, or what a search of
/// the network finds.
pub(super) async fn apps(addr: Option<SocketAddr>) -> Result<HashMap<String, SocketAddr>> {
    match addr {
        Some(addr) => Ok(HashMap::from([(GIVEN.to_string(), addr)])),
        None => discover().await,
    }
}

/// With no given address it tries the address cached by the last discovery
/// first and falls back to a fresh mDNS browse, so repeat calls skip the
/// discovery wait.
pub(super) async fn connect(app: Option<String>, addr: Option<SocketAddr>) -> Result<Client> {
    if let Some(addr) = addr {
        return match timeout(CONNECT, Client::connect(addr)).await {
            Ok(client) => client.with_context(|| format!("No app answers at {addr}")),
            Err(elapsed) => {
                bail!("No app answers at {addr}: {elapsed}. The app must run in front on an unlocked device.")
            }
        };
    }

    if let Some(addr) = cached_addr(app.as_deref())
        && let Ok(Ok(client)) = timeout(Duration::from_secs(1), Client::connect(addr)).await
    {
        return Ok(client);
    }

    let apps = discover().await?;
    save_cache(&apps)?;
    let addr = resolve(&apps, app)?;

    Client::connect(addr).await
}

fn cache_path() -> PathBuf {
    temp_dir().join("hilen-inspect-apps.json")
}

fn cached_addr(app: Option<&str>) -> Option<SocketAddr> {
    let cache: HashMap<String, SocketAddr> = from_str(&read_to_string(cache_path()).ok()?).ok()?;
    match app {
        Some(id) => cache.get(id).copied(),
        // A single cached app can be trusted without a browse. With several,
        // discover every time, correctness over speed.
        None => {
            if cache.len() == 1 {
                cache.values().next().copied()
            } else {
                None
            }
        }
    }
}

pub(super) fn save_cache(apps: &HashMap<String, SocketAddr>) -> Result<()> {
    write(cache_path(), to_string(apps)?)?;
    Ok(())
}

fn resolve(apps: &HashMap<String, SocketAddr>, app: Option<String>) -> Result<SocketAddr> {
    let ids = || apps.keys().cloned().collect::<Vec<_>>().join(", ");

    if let Some(id) = app {
        return match apps.get(&id) {
            Some(addr) => Ok(*addr),
            None => bail!("App {id} not found. Running apps: {}", ids()),
        };
    }

    match apps.len() {
        0 => bail!(NO_APPS),
        1 => Ok(*apps.values().next().unwrap()),
        _ => bail!("Multiple apps running, pass --app. Running apps: {}", ids()),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hilen::inspect::protocol::FIXED_PORT;
    use tokio::runtime::Builder;

    use super::{given, has_port};

    #[test]
    fn a_host_with_no_port_gets_the_fixed_port() -> Result<()> {
        Builder::new_current_thread().enable_all().build()?.block_on(async {
            let bare = given(Some("100.81.24.110".to_string())).await?;
            assert_eq!(bare, Some(format!("100.81.24.110:{FIXED_PORT}").parse()?));
            let full = given(Some(" 100.81.24.110:5000 ".to_string())).await?;
            assert_eq!(full, Some("100.81.24.110:5000".parse()?));
            let six = given(Some("fd7a::1".to_string())).await?;
            assert_eq!(six, Some(format!("[fd7a::1]:{FIXED_PORT}").parse()?));
            Ok(())
        })
    }

    #[test]
    fn only_a_number_after_the_host_is_a_port() {
        assert!(has_port("phone:7435"));
        assert!(has_port("[fd7a::1]:7435"));
        assert!(!has_port("phone"));
        assert!(!has_port("fd7a::1"));
        assert!(!has_port("phone:name"));
    }
}
