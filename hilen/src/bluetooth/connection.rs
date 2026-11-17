use std::{
    fmt::{self, Debug, Formatter},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, anyhow, bail};
use log::{debug, info, warn};
use parking_lot::Mutex;
use web_time::Instant;

use crate::{
    bluetooth::{
        BleCharacteristic, BleDevice, BleNotification, BleProperty, BleService, BleWrite, Uuid,
        backend::{Connected, Link},
    },
    deps::hreads::on_main,
};

type NotificationAction = Box<dyn FnMut(BleNotification) + Send>;
type DisconnectAction = Box<dyn FnOnce() + Send>;

/// An open connection to a device, from `Bluetooth::connect`. Dropping it
/// disconnects.
pub struct BleConnection {
    device:   BleDevice,
    services: Vec<BleService>,
    link:     Box<dyn Link>,
    events:   Events,
    /// `disconnect` ran, the drop has nothing left to do.
    done:     bool,
}

impl BleConnection {
    pub(super) fn new(device: BleDevice, connected: Connected, events: Events) -> Self {
        let uuids: Vec<_> = connected.services.iter().map(|service| service.uuid).collect();
        info!("BLE {} is connected, services {uuids:?}", device.label());
        Self {
            device,
            services: connected.services,
            link: connected.link,
            events,
            done: false,
        }
    }

    pub fn device(&self) -> &BleDevice {
        &self.device
    }

    /// The services the connect found, with their characteristics.
    pub fn services(&self) -> &[BleService] {
        &self.services
    }

    /// The characteristic with this uuid, in the first service that has it.
    pub fn characteristic(&self, uuid: Uuid) -> Option<&BleCharacteristic> {
        self.services
            .iter()
            .flat_map(|service| &service.characteristics)
            .find(|characteristic| characteristic.uuid == uuid)
    }

    /// False once the device went away. Then every call fails, connect again.
    pub fn is_connected(&self) -> bool {
        self.events.0.connected.load(Ordering::Relaxed)
    }

    pub async fn read(&self, characteristic: Uuid) -> Result<Vec<u8>> {
        let found = self.allowed(characteristic, &[BleProperty::Read], "read")?;
        let start = Instant::now();
        let value = self.link.read(found).await;
        self.log("read", characteristic, start, &value);
        value
    }

    pub async fn write(&self, characteristic: Uuid, value: &[u8], kind: BleWrite) -> Result<()> {
        let property = match kind {
            BleWrite::WithResponse => BleProperty::Write,
            BleWrite::WithoutResponse => BleProperty::WriteWithoutResponse,
        };
        let found = self.allowed(characteristic, &[property], "write")?;
        let start = Instant::now();
        let done = self.link.write(found, value.to_vec(), kind).await;
        self.log(
            &format!("write of {} bytes", value.len()),
            characteristic,
            start,
            &done,
        );
        done
    }

    /// The device sends each new value of `characteristic`, `on_notification`
    /// gets them.
    pub async fn subscribe(&self, characteristic: Uuid) -> Result<()> {
        let found = self.allowed(
            characteristic,
            &[BleProperty::Notify, BleProperty::Indicate],
            "notify",
        )?;
        let start = Instant::now();
        let done = self.link.subscribe(found).await;
        self.log("subscribe", characteristic, start, &done);
        done
    }

    pub async fn unsubscribe(&self, characteristic: Uuid) -> Result<()> {
        let found = self.allowed(
            characteristic,
            &[BleProperty::Notify, BleProperty::Indicate],
            "notify",
        )?;
        let start = Instant::now();
        let done = self.link.unsubscribe(found).await;
        self.log("unsubscribe", characteristic, start, &done);
        done
    }

    /// Calls `action` on the main thread for every notification, in the
    /// order they arrived. A new action takes the place of the old one. A
    /// notification that arrives while there is no action is dropped.
    pub fn on_notification(&self, action: impl FnMut(BleNotification) + Send + 'static) {
        *self.events.0.notification.lock() = Some(Box::new(action));
    }

    /// Calls `action` on the main thread once, when the device goes away
    /// while the connection is open. A drop or `disconnect` does not call it.
    pub fn on_disconnect(&self, action: impl FnOnce() + Send + 'static) {
        *self.events.0.disconnect.lock() = Some(Box::new(action));
    }

    /// Disconnects and waits for the system to finish it.
    pub async fn disconnect(mut self) -> Result<()> {
        self.events.close();
        self.done = true;
        let done = self.link.disconnect().await;
        match &done {
            Ok(()) => info!("BLE {} is disconnected", self.device.label()),
            Err(err) => warn!("BLE disconnect of {} failed: {err}", self.device.label()),
        }
        done
    }

    /// The characteristic, when the device has it and it allows 1 of
    /// `properties`.
    fn allowed(&self, uuid: Uuid, properties: &[BleProperty], what: &str) -> Result<BleCharacteristic> {
        if !self.is_connected() {
            bail!("BLE {} is not connected", self.device.label());
        }
        let characteristic = self
            .characteristic(uuid)
            .ok_or_else(|| anyhow!("BLE {} has no characteristic {uuid}", self.device.label()))?;
        if !properties.iter().any(|property| characteristic.can(*property)) {
            bail!(
                "BLE characteristic {uuid} of {} does not allow {what}, it allows {:?}",
                self.device.label(),
                characteristic.properties
            );
        }
        Ok(characteristic.clone())
    }

    fn log<T>(&self, what: &str, characteristic: Uuid, start: Instant, result: &Result<T>) {
        let device = self.device.label();
        let time = start.elapsed();
        match result {
            Ok(_) => debug!("BLE {what} of {characteristic} on {device} done in {time:?}"),
            Err(err) => warn!("BLE {what} of {characteristic} on {device} failed after {time:?}: {err}"),
        }
    }
}

impl Debug for BleConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BleConnection")
            .field("device", &self.device)
            .field("connected", &self.is_connected())
            .finish_non_exhaustive()
    }
}

impl Drop for BleConnection {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        self.events.close();
        self.link.close();
        info!("BLE {} is closed", self.device.label());
    }
}

/// What a backend reports about an open connection, from any thread. Each
/// report goes to the main thread.
#[derive(Clone)]
pub(super) struct Events(Arc<Subscriber>);

struct Subscriber {
    device:       String,
    notification: Mutex<Option<NotificationAction>>,
    disconnect:   Mutex<Option<DisconnectAction>>,
    connected:    AtomicBool,
    /// A report can still wait for the main thread when the connection is
    /// dropped. Its action must not run then, the view it calls is gone.
    closed:       AtomicBool,
}

impl Events {
    pub(super) fn new(device: &str) -> Self {
        Self(Arc::new(Subscriber {
            device:       device.to_string(),
            notification: Mutex::new(None),
            disconnect:   Mutex::new(None),
            connected:    AtomicBool::new(true),
            closed:       AtomicBool::new(false),
        }))
    }

    pub(super) fn notify(&self, characteristic: Uuid, value: Vec<u8>) {
        if self.0.closed.load(Ordering::Relaxed) {
            return;
        }
        let subscriber = self.0.clone();
        on_main(move || {
            subscriber.deliver(BleNotification {
                characteristic,
                value,
            });
        });
    }

    /// The device went away. Only the first report counts.
    pub(super) fn lost(&self) {
        if self.0.closed.load(Ordering::Relaxed) || !self.0.connected.swap(false, Ordering::Relaxed) {
            return;
        }
        let subscriber = self.0.clone();
        on_main(move || subscriber.lose());
    }

    fn close(&self) {
        self.0.closed.store(true, Ordering::Relaxed);
        self.0.connected.store(false, Ordering::Relaxed);
        self.0.notification.lock().take();
        self.0.disconnect.lock().take();
    }
}

impl Subscriber {
    fn deliver(&self, notification: BleNotification) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        // The action is out of the lock while it runs, so it can call
        // `on_notification` itself.
        let Some(mut action) = self.notification.lock().take() else {
            debug!(
                "BLE notification of {} from {} dropped, no action",
                notification.characteristic, self.device
            );
            return;
        };
        action(notification);
        if !self.closed.load(Ordering::Relaxed) {
            self.notification.lock().get_or_insert(action);
        }
    }

    fn lose(&self) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        warn!("BLE {} lost the connection", self.device);
        // Out of the lock, the action can set a new one.
        let action = self.disconnect.lock().take();
        if let Some(action) = action {
            action();
        }
    }
}
