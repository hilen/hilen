//! The paired iPhones, iPads and Apple TVs of this Mac, as `devicectl` of
//! Xcode lists them. `--device` reaches an app on one of them with no
//! search of the network, so a command never lands on another device that
//! happens to run a hilen app.
//!
//! A device is asked over the tunnel Xcode keeps to it first. That address
//! leads to this device and to no other. An app built before the server
//! listened on the tunnel is asked by the name of the device on the local
//! network.

use std::{
    env::temp_dir,
    fs::{read_to_string, remove_file},
    net::{IpAddr, SocketAddr},
    process::{Command, Stdio, id},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use hilen::inspect::protocol::FIXED_PORT;
use serde::Deserialize;
use serde_json::from_str;
use tokio::{
    net::{TcpStream, lookup_host},
    time::timeout,
};

const CONNECT: Duration = Duration::from_secs(2);
const PAIRED: &str = "paired";
/// The end of the name a device has on the tunnel.
const TUNNEL_NAME: &str = ".coredevice.local";
const LOCAL_NAME: &str = ".local";

/// The JSON of `devicectl list devices`, only what is read here.
#[derive(Deserialize)]
struct List {
    result: Listed,
}

#[derive(Deserialize)]
struct Listed {
    devices: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    identifier:            String,
    #[serde(default)]
    connection_properties: Connection,
    #[serde(default)]
    device_properties:     Properties,
    #[serde(default)]
    hardware_properties:   Hardware,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Connection {
    #[serde(default)]
    pairing_state:     String,
    #[serde(default, rename = "tunnelIPAddress")]
    tunnel_ip_address: Option<IpAddr>,
    #[serde(default)]
    local_hostnames:   Vec<String>,
}

#[derive(Deserialize, Default)]
struct Properties {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Hardware {
    #[serde(default)]
    device_type: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Device {
    /// The id every `devicectl` command takes.
    id:     String,
    name:   String,
    /// `iPhone`, `iPad` or `appleTV`.
    kind:   String,
    tunnel: Option<IpAddr>,
    /// The name on the local network.
    host:   Option<String>,
}

impl Device {
    /// Where an app on the device answers, none when no app does.
    async fn addr(&self) -> Option<SocketAddr> {
        let mut tries: Vec<SocketAddr> =
            self.tunnel.map(|ip| SocketAddr::new(ip, FIXED_PORT)).into_iter().collect();
        if let Some(host) = &self.host
            && let Ok(found) = lookup_host((host.as_str(), FIXED_PORT)).await
        {
            let (four, six): (Vec<_>, Vec<_>) = found.partition(SocketAddr::is_ipv4);
            tries.extend(four);
            tries.extend(six);
        }
        for addr in tries {
            if let Ok(Ok(_)) = timeout(CONNECT, TcpStream::connect(addr)).await {
                return Some(addr);
            }
        }
        None
    }

    fn line(&self) -> String {
        format!("{} ({}, {})", self.name, self.kind, self.id)
    }
}

fn parse(json: &str) -> Result<Vec<Device>> {
    let list: List = from_str(json).context("devicectl printed no list of devices")?;
    Ok(list
        .result
        .devices
        .into_iter()
        .filter(|entry| entry.connection_properties.pairing_state == PAIRED)
        .map(|entry| Device {
            id:     entry.identifier,
            name:   entry.device_properties.name,
            kind:   entry.hardware_properties.device_type,
            tunnel: entry.connection_properties.tunnel_ip_address,
            host:   entry
                .connection_properties
                .local_hostnames
                .first()
                .and_then(|name| name.strip_suffix(TUNNEL_NAME))
                .map(|name| format!("{name}{LOCAL_NAME}")),
        })
        .collect())
}

/// The device `word` names: its whole id, or a part of its name or of its
/// kind, like `tv`, `ipad` or `bedroom`. Letter case does not count.
fn pick(mut devices: Vec<Device>, word: &str) -> Result<Device> {
    let all: Vec<String> = devices.iter().map(Device::line).collect();
    let word = word.to_lowercase();
    devices.retain(|device| {
        device.id.to_lowercase() == word
            || device.name.to_lowercase().contains(&word)
            || device.kind.to_lowercase().contains(&word)
    });
    match devices.len() {
        1 => Ok(devices.remove(0)),
        0 if all.is_empty() => bail!("No paired device. Plug it in, unlock it and tap Trust."),
        0 => bail!("No paired device matches `{word}`. Paired: {}", all.join(", ")),
        _ => bail!(
            "Several paired devices match `{word}`, name 1 of them. Paired: {}",
            all.join(", ")
        ),
    }
}

fn paired() -> Result<Vec<Device>> {
    let file = temp_dir().join(format!("hilen-inspect-devices-{}.json", id()));
    let status = Command::new("xcrun")
        .args(["devicectl", "list", "devices", "--quiet", "--json-output"])
        .arg(&file)
        .stdout(Stdio::null())
        .status()
        .context("xcrun did not start, the devices of this Mac are listed by Xcode")?;
    if !status.success() {
        bail!("`xcrun devicectl list devices` failed");
    }
    let json = read_to_string(&file)?;
    remove_file(&file)?;
    parse(&json)
}

/// The address of the app on the device `word` names.
pub(super) async fn reach(word: &str) -> Result<SocketAddr> {
    let device = pick(paired()?, word)?;
    match device.addr().await {
        Some(addr) => Ok(addr),
        None => bail!(
            "{} is paired, but no app answers on port {FIXED_PORT}. The app must run in front on an unlocked device.",
            device.line()
        ),
    }
}

/// Prints the paired devices, or only the one `word` names, 1 per line:
/// the id, the kind, the name and the address an app answers at, with a
/// tab between them. The address is `-` when no app answers.
pub(super) async fn print(word: Option<&str>) -> Result<()> {
    let devices = paired()?;
    let devices = match word {
        Some(word) => vec![pick(devices, word)?],
        None => devices,
    };
    for device in devices {
        let addr = device.addr().await.map_or_else(|| "-".to_string(), |addr| addr.to_string());
        println!("{}\t{}\t{}\t{addr}", device.id, device.kind, device.name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Device, parse, pick};

    const LIST: &str = r#"{"result": {"devices": [
{"identifier": "88F14386", "connectionProperties": {"pairingState": "paired", "tunnelIPAddress": "fd66:2f65:27a0::1",
"localHostnames": ["Bedroom-TV-1.coredevice.local", "88F14386.coredevice.local"]},
"deviceProperties": {"name": "Bedroom TV"}, "hardwareProperties": {"deviceType": "appleTV", "platform": "tvOS"}},
{"identifier": "1E96D689", "connectionProperties": {"pairingState": "paired", "localHostnames": []},
"deviceProperties": {"name": "My iPhone"}, "hardwareProperties": {"deviceType": "iPhone"}},
{"identifier": "0000AAAA", "connectionProperties": {"pairingState": "unpaired"},
"deviceProperties": {"name": "Old iPad"}, "hardwareProperties": {"deviceType": "iPad"}}
]}}"#;

    fn devices() -> Result<Vec<Device>> {
        parse(LIST)
    }

    #[test]
    fn only_paired_devices_are_listed() -> Result<()> {
        let devices = devices()?;
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Bedroom TV");
        assert_eq!(devices[0].tunnel, Some("fd66:2f65:27a0::1".parse()?));
        assert_eq!(devices[0].host.as_deref(), Some("Bedroom-TV-1.local"));
        assert_eq!(devices[1].tunnel, None);
        assert_eq!(devices[1].host, None);
        Ok(())
    }

    #[test]
    fn a_device_is_picked_by_its_id_its_name_or_its_kind() -> Result<()> {
        assert_eq!(pick(devices()?, "1e96d689")?.name, "My iPhone");
        assert_eq!(pick(devices()?, "bedroom")?.kind, "appleTV");
        assert_eq!(pick(devices()?, "TV")?.id, "88F14386");
        assert_eq!(pick(devices()?, "iphone")?.id, "1E96D689");
        Ok(())
    }

    #[test]
    fn a_word_that_fits_no_device_or_several_is_refused() -> Result<()> {
        let none = pick(devices()?, "ipad").unwrap_err().to_string();
        assert!(none.contains("No paired device matches"), "{none}");
        // Both names have an `e`.
        let several = pick(devices()?, "e").unwrap_err().to_string();
        assert!(several.contains("Several paired devices"), "{several}");
        Ok(())
    }
}
