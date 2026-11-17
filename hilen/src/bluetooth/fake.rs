use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use parking_lot::Mutex;

use crate::bluetooth::{
    BleCharacteristic, BleDevice, BleService, BleWrite, Uuid,
    backend::{Backend, Connected, Link, Pending},
    connection::Events,
};

/// Devices a test puts in range, and the values and calls they hold.
#[derive(Clone, Default)]
pub(super) struct FakeBluetooth {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    devices:    Vec<(BleDevice, Vec<BleService>)>,
    values:     HashMap<Uuid, Vec<u8>>,
    written:    Vec<(Uuid, Vec<u8>, BleWrite)>,
    subscribed: Vec<Uuid>,
    open:       Vec<(String, Events)>,
}

impl FakeBluetooth {
    pub fn add(&self, device: BleDevice, services: Vec<BleService>) {
        self.state.lock().devices.push((device, services));
    }

    pub fn set_value(&self, characteristic: Uuid, value: &[u8]) {
        self.state.lock().values.insert(characteristic, value.to_vec());
    }

    pub fn written(&self) -> Vec<(Uuid, Vec<u8>, BleWrite)> {
        self.state.lock().written.clone()
    }

    pub fn subscribed(&self) -> Vec<Uuid> {
        self.state.lock().subscribed.clone()
    }

    pub fn open_count(&self) -> usize {
        self.state.lock().open.len()
    }

    /// The device sends a new value, on the calling thread, the way a
    /// system does on a thread of its own.
    pub fn notify(&self, device: &BleDevice, characteristic: Uuid, value: &[u8]) {
        let receivers: Vec<_> = {
            let state = self.state.lock();
            if !state.subscribed.contains(&characteristic) {
                return;
            }
            state
                .open
                .iter()
                .filter(|(id, _)| *id == device.id)
                .map(|(_, events)| events.clone())
                .collect()
        };
        for events in receivers {
            events.notify(characteristic, value.to_vec());
        }
    }

    /// The device goes out of range.
    pub fn lose(&self, device: &BleDevice) {
        let lost: Vec<_> = {
            let mut state = self.state.lock();
            let (lost, kept) = state.open.drain(..).partition(|(id, _)| *id == device.id);
            state.open = kept;
            lost
        };
        for (_, events) in lost {
            events.lost();
        }
    }
}

impl Backend for FakeBluetooth {
    fn scan(&self, services: Vec<Uuid>, _time: Duration) -> Pending<'_, Vec<BleDevice>> {
        let devices = self
            .state
            .lock()
            .devices
            .iter()
            .map(|(device, _)| device)
            .filter(|device| {
                services.is_empty() || device.services.iter().any(|uuid| services.contains(uuid))
            })
            .cloned()
            .collect();
        Box::pin(async move { Ok(devices) })
    }

    fn connect(&self, device: BleDevice, events: Events) -> Pending<'_, Connected> {
        let mut state = self.state.lock();
        let services = state
            .devices
            .iter()
            .find(|(known, _)| known.id == device.id)
            .map(|(_, services)| services.clone())
            .ok_or_else(|| anyhow!("BLE {} is not in range", device.label()));
        let connected = services.map(|services| {
            state.open.push((device.id.clone(), events));
            Connected {
                link: Box::new(FakeLink {
                    id:    device.id.clone(),
                    state: self.state.clone(),
                }),
                services,
            }
        });
        Box::pin(async move { connected })
    }
}

struct FakeLink {
    id:    String,
    state: Arc<Mutex<State>>,
}

impl FakeLink {
    fn answer<T: Send + 'static>(answer: Result<T>) -> Pending<'static, T> {
        Box::pin(async move { answer })
    }
}

impl Link for FakeLink {
    fn read(&self, characteristic: BleCharacteristic) -> Pending<'_, Vec<u8>> {
        let value = self.state.lock().values.get(&characteristic.uuid).cloned();
        Self::answer(value.ok_or_else(|| anyhow!("no value for {}", characteristic.uuid)))
    }

    fn write(&self, characteristic: BleCharacteristic, value: Vec<u8>, kind: BleWrite) -> Pending<'_, ()> {
        self.state.lock().written.push((characteristic.uuid, value, kind));
        Self::answer(Ok(()))
    }

    fn subscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        self.state.lock().subscribed.push(characteristic.uuid);
        Self::answer(Ok(()))
    }

    fn unsubscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        self.state.lock().subscribed.retain(|uuid| *uuid != characteristic.uuid);
        Self::answer(Ok(()))
    }

    fn disconnect(&self) -> Pending<'_, ()> {
        self.close();
        Self::answer(Ok(()))
    }

    fn close(&self) {
        self.state.lock().open.retain(|(id, _)| *id != self.id);
    }
}
