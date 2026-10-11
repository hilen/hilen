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
use hilen::inspect::protocol::{FIXED_PORT, discover};
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
    pairing_state:       String,
    #[serde(default, rename = "tunnelIPAddress")]
    tunnel_ip_address:   Option<IpAddr>,
    #[serde(default)]
    local_hostnames:     Vec<String>,
    /// The names of a device with no tunnel open, where the list above is
    /// empty.
    #[serde(default)]
    potential_hostnames: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Properties {
    #[serde(default)]
    name:              String,
    #[serde(default)]
    os_version_number: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Hardware {
    #[serde(default)]
    device_type: String,
    #[serde(default)]
    platform:    String,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Device {
    /// The id every `devicectl` command takes.
    id:     String,
    name:   String,
    /// `iPhone`, `iPad` or `appleTV`.
    kind:   String,
    /// The system and its version, like `tvOS 26.3`.
    os:     String,
    tunnel: Option<IpAddr>,
    /// The name on the local network.
    host:   Option<String>,
}

/// The names a device may have on the local network, from the name Xcode
/// has for it. Xcode adds a number to its own name when it saw the name
/// before, `Bedroom-TV-1`, while the device still calls itself
/// `Bedroom-TV`, so the name with that number cut off is tried too.
fn local_names(host: &str) -> Vec<String> {
    let mut names = vec![host.to_string()];
    if let Some(stem) = host.strip_suffix(LOCAL_NAME)
        && let Some((name, number)) = stem.rsplit_once('-')
        && !number.is_empty()
        && number.chars().all(|digit| digit.is_ascii_digit())
    {
        names.push(format!("{name}{LOCAL_NAME}"));
    }
    names
}

impl Device {
    /// The addresses of the device with the fixed port, the tunnel first.
    async fn bases(&self) -> Vec<SocketAddr> {
        let mut bases: Vec<SocketAddr> =
            self.tunnel.map(|ip| SocketAddr::new(ip, FIXED_PORT)).into_iter().collect();
        for name in self.host.as_deref().map_or_else(Vec::new, local_names) {
            if let Ok(found) = lookup_host((name.as_str(), FIXED_PORT)).await {
                let (four, six): (Vec<_>, Vec<_>) = found.partition(SocketAddr::is_ipv4);
                bases.extend(four);
                bases.extend(six);
            }
        }
        bases
    }

    /// Every address an app on the device may answer at. The first app on
    /// a device takes the fixed port. One that started while that port was
    /// still held took another, which only a search of the network tells.
    /// `search` adds the ports of the apps the search finds at the device,
    /// after the fixed port.
    pub(super) async fn places(&self, search: bool) -> Result<Vec<SocketAddr>> {
        let bases = self.bases().await;
        let mut ports = vec![FIXED_PORT];
        if search {
            let apps: Vec<SocketAddr> = discover().await?.into_values().collect();
            ports.extend(ports_at(&bases, &apps));
        }
        Ok(with_ports(&bases, &ports))
    }

    /// Where an app on the device answers, none when no app does.
    async fn addr(&self) -> Result<Option<SocketAddr>> {
        for search in [false, true] {
            for addr in self.places(search).await? {
                // The fixed port was asked in the first round.
                if search && addr.port() == FIXED_PORT {
                    continue;
                }
                if let Ok(Ok(_)) = timeout(CONNECT, TcpStream::connect(addr)).await {
                    return Ok(Some(addr));
                }
            }
        }
        Ok(None)
    }

    pub(super) fn line(&self) -> String {
        format!("{} ({}, {}, {})", self.name, self.kind, self.os, self.id)
    }
}

/// The ports, besides the fixed one, of the found `apps` that run at one of
/// the addresses in `bases`.
fn ports_at(bases: &[SocketAddr], apps: &[SocketAddr]) -> Vec<u16> {
    let mut ports = vec![];
    for app in apps {
        let here = bases.iter().any(|base| base.ip() == app.ip());
        if here && app.port() != FIXED_PORT && !ports.contains(&app.port()) {
            ports.push(app.port());
        }
    }
    ports
}

/// Every address of `bases` with every port of `ports`, all addresses of
/// the first port first. A base keeps the rest of itself, an IPv6 address
/// of the local link needs its scope.
fn with_ports(bases: &[SocketAddr], ports: &[u16]) -> Vec<SocketAddr> {
    let mut places = vec![];
    for port in ports {
        for base in bases {
            let mut place = *base;
            place.set_port(*port);
            places.push(place);
        }
    }
    places
}

fn parse(json: &str) -> Result<Vec<Device>> {
    let list: List = from_str(json).context("devicectl printed no list of devices")?;
    Ok(list
        .result
        .devices
        .into_iter()
        .filter(|entry| entry.connection_properties.pairing_state == PAIRED)
        .map(|entry| {
            let connection = entry.connection_properties;
            let os = format!(
                "{} {}",
                entry.hardware_properties.platform, entry.device_properties.os_version_number
            );
            Device {
                id:     entry.identifier,
                name:   entry.device_properties.name,
                kind:   entry.hardware_properties.device_type,
                os:     os.trim().to_string(),
                tunnel: connection.tunnel_ip_address,
                host:   connection
                    .local_hostnames
                    .first()
                    .or_else(|| connection.potential_hostnames.first())
                    .and_then(|name| name.strip_suffix(TUNNEL_NAME))
                    .map(|name| format!("{name}{LOCAL_NAME}")),
            }
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

/// The paired device `word` names.
pub(super) fn named(word: &str) -> Result<Device> {
    pick(paired()?, word)
}

/// The address of the app on the device `word` names.
pub(super) async fn reach(word: &str) -> Result<SocketAddr> {
    let device = named(word)?;
    match device.addr().await? {
        Some(addr) => Ok(addr),
        None => bail!(
            "{} is paired, but no app answers on it. The app must run in front on an unlocked device.",
            device.line()
        ),
    }
}

/// Prints the paired devices, or only the one `word` names, 1 per line:
/// the id, the kind, the name, the system with its version and the address
/// an app answers at, with a tab between them. The address is `-` when no
/// app answers.
pub(super) async fn print(word: Option<&str>) -> Result<()> {
    let devices = paired()?;
    let devices = match word {
        Some(word) => vec![pick(devices, word)?],
        None => devices,
    };
    for device in devices {
        let addr = device.addr().await?.map_or_else(|| "-".to_string(), |addr| addr.to_string());
        println!(
            "{}\t{}\t{}\t{}\t{addr}",
            device.id, device.kind, device.name, device.os
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use anyhow::Result;
    use hilen::inspect::protocol::FIXED_PORT;

    use super::{Device, local_names, parse, pick, ports_at, with_ports};

    #[test]
    fn the_number_xcode_adds_to_a_name_is_also_tried_without() {
        assert_eq!(
            local_names("Bedroom-TV-1.local"),
            ["Bedroom-TV-1.local", "Bedroom-TV.local"]
        );
        assert_eq!(local_names("vladass-iPhone.local"), ["vladass-iPhone.local"]);
        assert_eq!(local_names("Apple-TV-4K.local"), ["Apple-TV-4K.local"]);
    }

    const LIST: &str = r#"{"result": {"devices": [
{"identifier": "88F14386", "connectionProperties": {"pairingState": "paired", "tunnelIPAddress": "fd66:2f65:27a0::1",
"localHostnames": ["Bedroom-TV-1.coredevice.local", "88F14386.coredevice.local"]},
"deviceProperties": {"name": "Bedroom TV", "osVersionNumber": "26.3"},
"hardwareProperties": {"deviceType": "appleTV", "platform": "tvOS"}},
{"identifier": "1E96D689", "connectionProperties": {"pairingState": "paired", "localHostnames": []},
"deviceProperties": {"name": "My iPhone"}, "hardwareProperties": {"deviceType": "iPhone"}},
{"identifier": "2222BBBB", "connectionProperties": {"pairingState": "paired",
"potentialHostnames": ["Other-iPhone.coredevice.local"]},
"deviceProperties": {"name": "Work phone", "osVersionNumber": "27.0.1"},
"hardwareProperties": {"deviceType": "iPhone", "platform": "iOS"}},
{"identifier": "0000AAAA", "connectionProperties": {"pairingState": "unpaired"},
"deviceProperties": {"name": "Old iPad"}, "hardwareProperties": {"deviceType": "iPad"}}
]}}"#;

    fn devices() -> Result<Vec<Device>> {
        parse(LIST)
    }

    #[test]
    fn only_paired_devices_are_listed() -> Result<()> {
        let devices = devices()?;
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].name, "Bedroom TV");
        assert_eq!(devices[0].os, "tvOS 26.3");
        assert_eq!(devices[1].os, "");
        assert_eq!(devices[2].os, "iOS 27.0.1");
        assert_eq!(devices[0].tunnel, Some("fd66:2f65:27a0::1".parse()?));
        assert_eq!(devices[0].host.as_deref(), Some("Bedroom-TV-1.local"));
        assert_eq!(devices[1].tunnel, None);
        assert_eq!(devices[1].host, None);
        Ok(())
    }

    #[test]
    fn a_device_with_no_tunnel_open_still_has_its_name_on_the_network() -> Result<()> {
        assert_eq!(devices()?[2].host.as_deref(), Some("Other-iPhone.local"));
        Ok(())
    }

    #[test]
    fn an_app_on_another_port_is_found_only_at_the_device() -> Result<()> {
        let bases = ["[fd66::1]:7435".parse()?, "192.168.0.129:7435".parse()?];
        let apps = [
            "192.168.0.129:62484".parse()?,
            "192.168.0.129:7435".parse()?,
            "192.168.0.208:50001".parse()?,
        ];
        let ports = ports_at(&bases, &apps);
        assert_eq!(ports, [62484]);

        let places = with_ports(&bases, &[FIXED_PORT, 62484]);
        let expected: [SocketAddr; 4] = [
            "[fd66::1]:7435".parse()?,
            "192.168.0.129:7435".parse()?,
            "[fd66::1]:62484".parse()?,
            "192.168.0.129:62484".parse()?,
        ];
        assert_eq!(places, expected);
        Ok(())
    }

    #[test]
    fn a_device_is_picked_by_its_id_its_name_or_its_kind() -> Result<()> {
        assert_eq!(pick(devices()?, "1e96d689")?.name, "My iPhone");
        assert_eq!(pick(devices()?, "bedroom")?.kind, "appleTV");
        assert_eq!(pick(devices()?, "TV")?.id, "88F14386");
        assert_eq!(pick(devices()?, "my iphone")?.id, "1E96D689");
        Ok(())
    }

    #[test]
    fn a_word_that_fits_no_device_or_several_is_refused() -> Result<()> {
        let none = pick(devices()?, "ipad").unwrap_err().to_string();
        assert!(none.contains("No paired device matches"), "{none}");
        let several = pick(devices()?, "iphone").unwrap_err().to_string();
        assert!(several.contains("Several paired devices"), "{several}");
        Ok(())
    }
}
