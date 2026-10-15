use std::{future::Future, pin::Pin, time::Duration};

use anyhow::Result;

use crate::bluetooth::{BleCharacteristic, BleDevice, BleService, BleWrite, Uuid, connection::Events};

/// The answer of a call to a device. It runs on the async runtime.
#[cfg(not_wasm)]
pub(super) type Pending<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// A browser future holds JS objects, which never leave the main thread.
#[cfg(wasm)]
pub(super) type Pending<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

/// Where the devices come from: the system, or a fake one in a test.
pub(super) trait Backend: Send + Sync {
    fn scan(&self, services: Vec<Uuid>, time: Duration) -> Pending<'_, Vec<BleDevice>>;

    /// `events` gets the notifications and the loss of the connection, on
    /// any thread.
    fn connect(&self, device: BleDevice, events: Events) -> Pending<'_, Connected>;
}

pub(super) struct Connected {
    pub link:     Box<dyn Link>,
    pub services: Vec<BleService>,
}

/// An open connection to 1 device.
pub(super) trait Link: Send + Sync {
    fn read(&self, characteristic: BleCharacteristic) -> Pending<'_, Vec<u8>>;

    fn write(&self, characteristic: BleCharacteristic, value: Vec<u8>, kind: BleWrite) -> Pending<'_, ()>;

    fn subscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()>;

    fn unsubscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()>;

    fn disconnect(&self) -> Pending<'_, ()>;

    /// The disconnect of a drop, nothing waits for it.
    fn close(&self);
}
