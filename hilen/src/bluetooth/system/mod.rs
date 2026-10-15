//! The devices of the system through btleplug: `CoreBluetooth` on macOS,
//! iOS and tvOS, `WinRT` on Windows, `BlueZ` on Linux and the Java
//! Bluetooth calls on Android.

#[cfg(android)]
mod android;

use std::{collections::HashMap, pin::Pin, time::Duration};

use anyhow::{Result, anyhow, bail};
use btleplug::{
    api::{
        Central, CentralEvent, CentralState, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
        ScanFilter, ValueNotification, WriteType,
    },
    platform::{Adapter, Manager, Peripheral},
};
use futures_util::{Stream, StreamExt};
use log::{debug, info, warn};
use parking_lot::Mutex;
use tokio::{runtime::Handle, sync::OnceCell, task::JoinHandle, time::sleep};

use crate::bluetooth::{
    BleCharacteristic, BleDevice, BleProperty, BleService, BleWrite, Uuid,
    backend::{Backend, Connected, Link, Pending},
    connection::Events,
};

/// A system tells the state of its adapter a moment after the start. A
/// scan before that finds nothing on a Mac.
const POWER_WAIT: Duration = Duration::from_secs(3);
const POWER_STEP: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(super) struct System {
    adapter: OnceCell<Adapter>,
    /// Every device a scan found, by its id, for the connect.
    found:   Mutex<HashMap<String, Peripheral>>,
}

impl System {
    async fn adapter(&self) -> Result<&Adapter> {
        let adapter = self
            .adapter
            .get_or_try_init(|| async {
                #[cfg(android)]
                android::load()?;
                let manager = Manager::new().await?;
                let adapter = manager
                    .adapters()
                    .await?
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("This device has no Bluetooth adapter"))?;
                match adapter.adapter_info().await {
                    Ok(name) => info!("BLE adapter {name}"),
                    Err(err) => debug!("BLE adapter has no name: {err}"),
                }
                anyhow::Ok(adapter)
            })
            .await?;
        #[cfg(android)]
        android::ask_permissions().await?;
        Ok(adapter)
    }
}

/// Fails when Bluetooth is off. A system that does not tell its state goes
/// on.
async fn wait_for_power(adapter: &Adapter) -> Result<()> {
    let mut waited = Duration::ZERO;
    loop {
        match adapter.adapter_state().await {
            Ok(CentralState::PoweredOn) => return Ok(()),
            Ok(CentralState::PoweredOff) => bail!("Bluetooth is turned off"),
            Ok(CentralState::Unknown) => {}
            Err(err) => {
                debug!("BLE adapter state is not known, going on: {err}");
                return Ok(());
            }
        }
        if waited >= POWER_WAIT {
            debug!("BLE adapter state is still unknown after {POWER_WAIT:?}, going on");
            return Ok(());
        }
        sleep(POWER_STEP).await;
        waited += POWER_STEP;
    }
}

impl Backend for System {
    fn scan(&self, services: Vec<Uuid>, time: Duration) -> Pending<'_, Vec<BleDevice>> {
        Box::pin(async move {
            let adapter = self.adapter().await?;
            wait_for_power(adapter).await?;
            adapter
                .start_scan(ScanFilter {
                    services: services.clone(),
                })
                .await?;
            sleep(time).await;
            adapter.stop_scan().await?;

            let mut devices = Vec::new();
            let mut found = self.found.lock().clone();
            for peripheral in adapter.peripherals().await? {
                let properties = match peripheral.properties().await {
                    Ok(Some(properties)) => properties,
                    Ok(None) => continue,
                    Err(err) => {
                        debug!("BLE device {} skipped, no properties: {err}", peripheral.id());
                        continue;
                    }
                };
                // Some systems give every device they ever heard, and a
                // Linux filter lets other ones through.
                if !services.is_empty() && !properties.services.iter().any(|uuid| services.contains(uuid)) {
                    continue;
                }
                let device = BleDevice {
                    id:       peripheral.id().to_string(),
                    name:     properties.local_name,
                    rssi:     properties.rssi,
                    services: properties.services,
                };
                found.insert(device.id.clone(), peripheral);
                devices.push(device);
            }
            *self.found.lock() = found;
            Ok(devices)
        })
    }

    fn connect(&self, device: BleDevice, events: Events) -> Pending<'_, Connected> {
        Box::pin(async move {
            let adapter = self.adapter().await?;
            let peripheral = self
                .found
                .lock()
                .get(&device.id)
                .cloned()
                .ok_or_else(|| anyhow!("BLE {} was not found by a scan", device.label()))?;
            if !peripheral.is_connected().await? {
                peripheral.connect().await?;
            }
            peripheral.discover_services().await?;
            let services = peripheral
                .services()
                .into_iter()
                .map(|service| BleService {
                    uuid:            service.uuid,
                    characteristics: service.characteristics.iter().map(characteristic).collect(),
                })
                .collect();

            let notifications = peripheral.notifications().await?;
            let lost = adapter.events().await?;
            let tasks = vec![
                tokio::spawn(forward_notifications(notifications, events.clone())),
                tokio::spawn(watch_loss(lost, peripheral.clone(), events)),
            ];
            Ok(Connected {
                link: Box::new(SystemLink {
                    peripheral,
                    label: device.label().to_string(),
                    tasks,
                }),
                services,
            })
        })
    }
}

/// The form btleplug gives its streams in.
type Feed<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

async fn forward_notifications(mut notifications: Feed<ValueNotification>, events: Events) {
    while let Some(notification) = notifications.next().await {
        events.notify(notification.uuid, notification.value);
    }
}

async fn watch_loss(mut central: Feed<CentralEvent>, peripheral: Peripheral, events: Events) {
    let id = peripheral.id();
    while let Some(event) = central.next().await {
        if matches!(&event, CentralEvent::DeviceDisconnected(gone) if *gone == id) {
            events.lost();
            return;
        }
    }
}

fn characteristic(characteristic: &Characteristic) -> BleCharacteristic {
    let flags = characteristic.properties;
    let properties = [
        (CharPropFlags::READ, BleProperty::Read),
        (CharPropFlags::WRITE, BleProperty::Write),
        (
            CharPropFlags::WRITE_WITHOUT_RESPONSE,
            BleProperty::WriteWithoutResponse,
        ),
        (CharPropFlags::NOTIFY, BleProperty::Notify),
        (CharPropFlags::INDICATE, BleProperty::Indicate),
    ]
    .into_iter()
    .filter(|(flag, _)| flags.contains(*flag))
    .map(|(_, property)| property)
    .collect();
    BleCharacteristic {
        uuid: characteristic.uuid,
        service: characteristic.service_uuid,
        properties,
    }
}

struct SystemLink {
    peripheral: Peripheral,
    label:      String,
    /// The forward of the notifications and the watch of a loss.
    tasks:      Vec<JoinHandle<()>>,
}

impl SystemLink {
    /// The characteristic of btleplug, with its descriptors.
    fn find(&self, wanted: &BleCharacteristic) -> Result<Characteristic> {
        self.peripheral
            .characteristics()
            .into_iter()
            .find(|found| found.uuid == wanted.uuid && found.service_uuid == wanted.service)
            .ok_or_else(|| anyhow!("BLE {} has no characteristic {}", self.label, wanted.uuid))
    }

    fn stop_tasks(&self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl Link for SystemLink {
    fn read(&self, characteristic: BleCharacteristic) -> Pending<'_, Vec<u8>> {
        Box::pin(async move { Ok(self.peripheral.read(&self.find(&characteristic)?).await?) })
    }

    fn write(&self, characteristic: BleCharacteristic, value: Vec<u8>, kind: BleWrite) -> Pending<'_, ()> {
        let kind = match kind {
            BleWrite::WithResponse => WriteType::WithResponse,
            BleWrite::WithoutResponse => WriteType::WithoutResponse,
        };
        Box::pin(async move { Ok(self.peripheral.write(&self.find(&characteristic)?, &value, kind).await?) })
    }

    fn subscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        Box::pin(async move { Ok(self.peripheral.subscribe(&self.find(&characteristic)?).await?) })
    }

    fn unsubscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        Box::pin(async move { Ok(self.peripheral.unsubscribe(&self.find(&characteristic)?).await?) })
    }

    fn disconnect(&self) -> Pending<'_, ()> {
        self.stop_tasks();
        Box::pin(async move { Ok(self.peripheral.disconnect().await?) })
    }

    fn close(&self) {
        self.stop_tasks();
        let Ok(runtime) = Handle::try_current() else {
            warn!(
                "BLE {} stays connected, no async runtime to disconnect on",
                self.label
            );
            return;
        };
        let peripheral = self.peripheral.clone();
        let label = self.label.clone();
        runtime.spawn(async move {
            if let Err(err) = peripheral.disconnect().await {
                warn!("BLE disconnect of {label} failed: {err}");
            }
        });
    }
}
