//! Bluetooth LE devices in the central role, the `bluetooth` feature.
//!
//! An app scans for devices with a service, connects to 1, reads and
//! writes its characteristics and gets their notifications.
//!
//! ```ignore
//! use hilen::bluetooth::{BleConnection, BleWrite, Bluetooth, Uuid};
//!
//! const SERVICE: Uuid = Uuid::from_u128(0x6e40_0001_b5a3_f393_e0a9_e50e_24dc_ca9e);
//! const COMMAND: Uuid = Uuid::from_u128(0x6e40_0002_b5a3_f393_e0a9_e50e_24dc_ca9e);
//! const STATUS: Uuid = Uuid::from_u128(0x6e40_0003_b5a3_f393_e0a9_e50e_24dc_ca9e);
//!
//! async fn open_board() -> anyhow::Result<Option<BleConnection>> {
//!     let devices = Bluetooth::scan(&[SERVICE], Duration::from_secs(5)).await?;
//!     let Some(board) = devices.first() else { return Ok(None) };
//!     let link = Bluetooth::connect(board).await?;
//!     link.subscribe(STATUS).await?;
//!     link.write(COMMAND, &[90], BleWrite::WithResponse).await?;
//!     Ok(Some(link))
//! }
//!
//! // In a view, `self` is its `Weak`.
//! spawn(async move {
//!     match open_board().await {
//!         Ok(Some(link)) => on_main(move || {
//!             link.on_notification(move |note| self.show(note.value));
//!             link.on_disconnect(move || self.lost());
//!             self.link = Some(link);
//!         }),
//!         Ok(None) => on_main(move || self.no_board()),
//!         Err(err) => on_main(move || self.failed(err)),
//!     }
//! });
//! ```
//!
//! - Every call that talks to the device is async. Run it inside `spawn` and go
//!   back to the main thread with `on_main`. The actions of `on_notification`
//!   and `on_disconnect` run on the main thread.
//! - macOS, iOS, tvOS, Windows, Linux and Android go through [btleplug](https://github.com/deviceplug/btleplug).
//!   Linux needs the `BlueZ` service. Android asks the user for the permissions
//!   at the first scan or connect, and loads the Java half of btleplug from a
//!   dex the engine carries, `make android-btleplug-dex` builds it.
//! - A browser goes through Web Bluetooth, Chrome and Edge have it. There a
//!   scan is the chooser of the browser, it ignores the time and gives the 1
//!   device the user picked, or none. Start it from a tap, the browser opens
//!   the chooser only right after a user gesture. Only the services of the scan
//!   can be used after the connect. Every call of a connection is for the main
//!   thread there.
//! - The texts the systems show when they ask for the permission come from
//!   `hilen.toml`:
//!
//! ```toml
//! [bluetooth]
//! usage = "The app drives its turntable over Bluetooth."
//! ```

mod backend;
#[cfg(test)]
mod bluetooth_test;
mod connection;
#[cfg(test)]
mod fake;
#[cfg(any(android, test))]
mod permissions;
#[cfg(not_wasm)]
mod system;
#[cfg(wasm)]
mod web;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use log::{info, warn};
use parking_lot::Mutex;
pub use uuid::Uuid;

pub use self::connection::BleConnection;
use self::{backend::Backend, connection::Events};

static BACKEND: Mutex<Option<Arc<dyn Backend>>> = Mutex::new(None);

/// A device a scan found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleDevice {
    /// The same for the same device while the app runs. What it is depends
    /// on the system, a Mac and an iPhone hide the address of the device.
    pub id:       String,
    pub name:     Option<String>,
    /// The signal strength in dBm, when the system tells it.
    pub rssi:     Option<i16>,
    /// The services the device advertises.
    pub services: Vec<Uuid>,
}

impl BleDevice {
    /// The name, or the id of a device with no name, for a log line.
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

/// A service of a connected device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleService {
    pub uuid:            Uuid,
    pub characteristics: Vec<BleCharacteristic>,
}

/// A value of a service that can be read, written or watched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleCharacteristic {
    pub uuid:       Uuid,
    pub service:    Uuid,
    pub properties: Vec<BleProperty>,
}

impl BleCharacteristic {
    pub fn can(&self, property: BleProperty) -> bool {
        self.properties.contains(&property)
    }
}

/// What a characteristic allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleProperty {
    Read,
    Write,
    WriteWithoutResponse,
    Notify,
    Indicate,
}

/// How a value is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleWrite {
    /// The device answers and the write waits for the answer.
    WithResponse,
    /// No answer, faster, a value can be lost.
    WithoutResponse,
}

/// A new value a device sent for a characteristic the app subscribed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleNotification {
    pub characteristic: Uuid,
    pub value:          Vec<u8>,
}

/// The Bluetooth LE devices around. Each call can run on any thread.
pub struct Bluetooth;

impl Bluetooth {
    /// Looks for devices that advertise one of `services` for `time`, and
    /// gives every one it heard. No services means every device. A browser
    /// shows its chooser in place of a scan, see the module doc.
    pub async fn scan(services: &[Uuid], time: Duration) -> Result<Vec<BleDevice>> {
        info!("BLE scan for {services:?} for {time:?}");
        let devices = backend()
            .scan(services.to_vec(), time)
            .await
            .inspect_err(|err| warn!("BLE scan for {services:?} failed: {err}"))?;
        let names: Vec<_> = devices.iter().map(BleDevice::label).collect();
        info!("BLE scan found {} devices: {names:?}", devices.len());
        Ok(devices)
    }

    /// Connects to `device` and finds its services. The connection stays
    /// until the `BleConnection` is dropped or the device goes away.
    pub async fn connect(device: &BleDevice) -> Result<BleConnection> {
        info!("BLE connect to {}", device.label());
        let events = Events::new(device.label());
        let connected = backend()
            .connect(device.clone(), events.clone())
            .await
            .inspect_err(|err| warn!("BLE connect to {} failed: {err}", device.label()))?;
        Ok(BleConnection::new(device.clone(), connected, events))
    }
}

fn backend() -> Arc<dyn Backend> {
    BACKEND.lock().get_or_insert_with(new_backend).clone()
}

#[cfg(not_wasm)]
fn new_backend() -> Arc<dyn Backend> {
    Arc::new(system::System::default())
}

#[cfg(wasm)]
fn new_backend() -> Arc<dyn Backend> {
    Arc::new(web::Web)
}
