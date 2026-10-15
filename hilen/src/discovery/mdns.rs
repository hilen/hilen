//! The browse through the `mdns-sd` crate, on Windows, Linux and Android.

use std::{collections::HashMap, net::IpAddr, thread::Builder};

use anyhow::Result;
use log::{debug, warn};
use mdns_sd::{Receiver, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent as MdnsEvent};

#[cfg(android)]
use super::android::MulticastLock;
use super::{
    NetService, Sender, ServiceEvent,
    service::{instance_name, sort_addresses},
};

pub(super) struct Browse {
    daemon: ServiceDaemon,
    /// Android drops multicast on Wi-Fi unless an app holds this.
    #[cfg(android)]
    lock:   Option<MulticastLock>,
}

impl Browse {
    pub(super) fn start(service_type: &str, sender: Sender) -> Result<Self> {
        #[cfg(android)]
        let lock = MulticastLock::acquire()?;

        let full_type = format!("{service_type}.local.");
        let daemon = ServiceDaemon::new()?;
        let events = daemon.browse(&full_type)?;
        Builder::new()
            .name("hilen-discovery".into())
            .spawn(move || forward(&events, &full_type, &sender))?;

        Ok(Self {
            daemon,
            #[cfg(android)]
            lock,
        })
    }

    /// The thread of the daemon ends, and with it the forward.
    pub(super) fn stop(&mut self) {
        if let Err(error) = self.daemon.shutdown() {
            warn!("The mDNS daemon did not stop: {error}");
        }
        #[cfg(android)]
        drop(self.lock.take());
    }
}

/// Turns the events of the daemon into the events of the browse, until the
/// daemon or the browser is gone.
fn forward(events: &Receiver<MdnsEvent>, full_type: &str, sender: &Sender) {
    let mut found = HashMap::<String, NetService>::new();
    while let Ok(event) = events.recv() {
        let event = match event {
            MdnsEvent::ServiceResolved(service) => {
                let Some(service) = net_service(*service, full_type) else {
                    continue;
                };
                if found.get(&service.name) == Some(&service) {
                    continue;
                }
                found.insert(service.name.clone(), service.clone());
                ServiceEvent::Found(service)
            }
            MdnsEvent::ServiceRemoved(_, full_name) => {
                let name = instance_name(&full_name, full_type);
                if found.remove(&name).is_none() {
                    continue;
                }
                ServiceEvent::Lost(name)
            }
            _ => continue,
        };
        if sender.send(Ok(event)).is_err() {
            debug!("The browse for {full_type} was dropped, its events end");
            return;
        }
    }
}

fn net_service(service: ResolvedService, full_type: &str) -> Option<NetService> {
    let mut addresses: Vec<IpAddr> = service.addresses.iter().map(ScopedIp::to_ip_addr).collect();
    if addresses.is_empty() {
        return None;
    }
    sort_addresses(&mut addresses);
    let text = service
        .txt_properties
        .iter()
        .map(|property| (property.key().to_string(), property.val_str().to_string()))
        .collect();
    Some(NetService {
        name: instance_name(&service.fullname, full_type),
        host: service.host,
        addresses,
        port: service.port,
        text,
    })
}
