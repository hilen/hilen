//! What a browse gives: the found services and the events, with the helpers
//! every backend shares.

use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};

use anyhow::{Result, ensure};

/// What a `ServiceBrowser` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEvent {
    /// A device announced the service. A service that was found already
    /// comes again when it changes, like a new address or a new text record.
    Found(NetService),
    /// The device with this name took the service off the network.
    Lost(String),
}

/// 1 device that announces the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetService {
    /// The name of this instance, like `Turntable 2`. The network keeps it
    /// unique for the service type.
    pub name:      String,
    /// The host name of the device, like `turntable-2.local.`.
    pub host:      String,
    /// Never empty. IPv4 comes first, an address of the real network before
    /// a link local one. An IPv6 link local address has no scope here, so it
    /// is last.
    pub addresses: Vec<IpAddr>,
    pub port:      u16,
    /// The text record. A key with no value has an empty text.
    pub text:      BTreeMap<String, String>,
}

impl NetService {
    /// The first address with the port, the one to connect to.
    pub fn socket_address(&self) -> SocketAddr {
        SocketAddr::new(self.addresses[0], self.port)
    }
}

/// The type of a service without its domain, like `_turnscan._tcp`. Takes it
/// with or without `.local.` at the end.
pub(crate) fn service_type(text: &str) -> Result<String> {
    let base = text.trim_end_matches('.').trim_end_matches(".local");
    let parts: Vec<&str> = base.split('.').collect();
    let valid = matches!(
        parts.as_slice(),
        [name, "_tcp" | "_udp"] if name.len() > 1 && name.starts_with('_')
    );
    ensure!(
        valid,
        "A service type looks like `_name._tcp` or `_name._udp`, `{text}` is not one"
    );
    Ok(base.to_string())
}

/// Puts the addresses in the order of `NetService::addresses` and drops the
/// ones that are there twice.
#[cfg(not_wasm)]
pub(crate) fn sort_addresses(addresses: &mut Vec<IpAddr>) {
    addresses.sort_by_key(|address| (rank(*address), *address));
    addresses.dedup();
}

#[cfg(not_wasm)]
fn rank(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(ip) if ip.is_link_local() => 1,
        IpAddr::V4(_) => 0,
        IpAddr::V6(ip) if ip.is_unicast_link_local() => 3,
        IpAddr::V6(_) => 2,
    }
}

/// The name of an instance out of its full name, `Board 2` out of
/// `Board 2._turnscan._tcp.local.`.
#[cfg(all(not_wasm, any(not(apple), test)))]
pub(crate) fn instance_name(full_name: &str, full_type: &str) -> String {
    full_name
        .strip_suffix(full_type)
        .and_then(|name| name.strip_suffix('.'))
        .unwrap_or(full_name)
        .to_string()
}

/// A DNS text record, a row of entries that each start with a length byte
/// and hold `key=value` or a `key` alone.
#[cfg(any(apple, all(test, not_wasm)))]
pub(crate) fn parse_text_record(record: &[u8]) -> BTreeMap<String, String> {
    let mut text = BTreeMap::new();
    let mut rest = record;
    while let Some((len, tail)) = rest.split_first() {
        let Some(entry) = tail.get(..usize::from(*len)) else {
            break;
        };
        rest = &tail[entry.len()..];
        if entry.is_empty() {
            continue;
        }
        let entry = String::from_utf8_lossy(entry);
        let (key, value) = entry.split_once('=').unwrap_or((&entry, ""));
        text.insert(key.to_string(), value.to_string());
    }
    text
}
