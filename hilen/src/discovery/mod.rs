//! Find devices on the local network by the service they announce over
//! mDNS, like `_turnscan._tcp`.
//!
//! macOS, iOS and tvOS ask the Bonjour service of the system. An iPhone
//! lets an app send multicast of its own only with an entitlement from
//! Apple, and on a Mac the `mdns-sd` crate shares the mDNS port with the
//! system and often hears no answer. Windows, Linux and Android use
//! `mdns-sd`, Android holds a Wi-Fi multicast lock while a browse runs. A
//! browser has no mDNS, there `ServiceBrowser::start` gives an error.

#[cfg(android)]
mod android;
#[cfg(apple)]
mod bonjour;
#[cfg(all(test, not_wasm))]
mod discovery_test;
#[cfg(all(not_wasm, not(apple)))]
mod mdns;
mod service;

#[cfg(wasm)]
use std::future::ready;

#[cfg(wasm)]
use anyhow::bail;
use anyhow::{Result, anyhow};
use log::debug;
#[cfg(not_wasm)]
use log::info;
#[cfg(not_wasm)]
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub use self::service::{NetService, ServiceEvent};

#[cfg(apple)]
type Source = bonjour::Browse;
#[cfg(all(not_wasm, not(apple)))]
type Source = mdns::Browse;

/// Where a backend puts what it finds. An error ends the browse.
#[cfg(not_wasm)]
type Sender = UnboundedSender<Result<ServiceEvent>>;

/// A running browse for 1 service type. It runs until it is dropped.
///
/// ```ignore
/// let mut browser = ServiceBrowser::start("_turnscan._tcp")?;
/// spawn(async move {
///     while let Ok(event) = browser.next_event().await {
///         on_main(move || match event {
///             ServiceEvent::Found(board) => self.show(board),
///             ServiceEvent::Lost(name) => self.remove(&name),
///         });
///     }
/// });
/// ```
pub struct ServiceBrowser {
    service_type: String,
    #[cfg(not_wasm)]
    events:       UnboundedReceiver<Result<ServiceEvent>>,
    #[cfg(not_wasm)]
    source:       Source,
}

impl ServiceBrowser {
    /// Starts to browse for `service_type`, like `_turnscan._tcp`, with or
    /// without `.local.` at the end. Any thread.
    pub fn start(service_type: &str) -> Result<Self> {
        let service_type = service::service_type(service_type)?;

        #[cfg(wasm)]
        bail!(
            "A browser has no mDNS, so it can not find {service_type} on the local network. The native app can."
        );

        #[cfg(not_wasm)]
        {
            let (sender, events) = unbounded_channel();
            let source = Source::start(&service_type, sender)?;
            info!("Browsing the local network for {service_type}");
            Ok(Self {
                service_type,
                events,
                source,
            })
        }
    }

    /// The type without its domain, like `_turnscan._tcp`.
    pub fn service_type(&self) -> &str {
        &self.service_type
    }

    /// A browser has no mDNS, so the browse there ends at once. It is no
    /// async fn, which would have nothing to await.
    #[cfg(wasm)]
    pub fn next_event(&mut self) -> impl Future<Output = Result<ServiceEvent>> {
        ready(Err(anyhow!(
            "A browser has no mDNS, {} is never found",
            self.service_type
        )))
    }

    /// The next device that came, changed or went. Waits until there is
    /// one. An error ends the browse, like the user not allowing the local
    /// network on an iPhone, and every later call gives an error too.
    #[cfg(not_wasm)]
    pub async fn next_event(&mut self) -> Result<ServiceEvent> {
        let event = self
            .events
            .recv()
            .await
            .ok_or_else(|| anyhow!("The browse for {} has ended", self.service_type))??;
        match &event {
            ServiceEvent::Found(service) => debug!(
                "Found {} of {} at {:?} port {}",
                service.name, self.service_type, service.addresses, service.port
            ),
            ServiceEvent::Lost(name) => debug!("Lost {name} of {}", self.service_type),
        }
        Ok(event)
    }
}

impl Drop for ServiceBrowser {
    fn drop(&mut self) {
        #[cfg(not_wasm)]
        self.source.stop();
        debug!("Stopped the browse for {}", self.service_type);
    }
}
