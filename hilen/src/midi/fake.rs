use std::sync::Arc;

use anyhow::{Result, anyhow};
use parking_lot::Mutex;

use crate::midi::{
    MidiDevice,
    backend::{Backend, Connection, Receive},
};

/// Devices a test plugs in, pulls out and sends bytes from.
#[derive(Clone, Default)]
pub(super) struct FakeMidi {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    devices: Vec<MidiDevice>,
    open:    Vec<(String, Arc<Mutex<Receive>>)>,
    plugged: usize,
}

impl FakeMidi {
    pub fn plug(&self, name: &str) -> MidiDevice {
        let mut state = self.state.lock();
        state.plugged += 1;
        let device = MidiDevice {
            id:   format!("fake-{}", state.plugged),
            name: name.to_string(),
        };
        state.devices.push(device.clone());
        device
    }

    pub fn unplug(&self, device: &MidiDevice) {
        let mut state = self.state.lock();
        state.devices.retain(|plugged| plugged != device);
        state.open.retain(|(id, _)| *id != device.id);
    }

    /// The device sends 1 message. It is handed over on the calling thread,
    /// the way a system hands a message over on a thread of its own.
    pub fn send(&self, device: &MidiDevice, bytes: &[u8]) {
        let receivers: Vec<_> = self
            .state
            .lock()
            .open
            .iter()
            .filter(|(id, _)| *id == device.id)
            .map(|(_, receive)| receive.clone())
            .collect();
        for receive in receivers {
            (receive.lock())(bytes);
        }
    }

    pub fn open_count(&self) -> usize {
        self.state.lock().open.len()
    }
}

impl Backend for FakeMidi {
    fn devices(&mut self) -> Vec<MidiDevice> {
        self.state.lock().devices.clone()
    }

    fn open(&mut self, device: &MidiDevice, receive: Receive) -> Result<Box<dyn Connection>> {
        let mut state = self.state.lock();
        if !state.devices.contains(device) {
            return Err(anyhow!("MIDI device {} is not plugged in", device.name));
        }
        let receive = Arc::new(Mutex::new(receive));
        state.open.push((device.id.clone(), receive.clone()));
        Ok(Box::new(FakeConnection {
            state: self.state.clone(),
            receive,
        }))
    }
}

struct FakeConnection {
    state:   Arc<Mutex<State>>,
    receive: Arc<Mutex<Receive>>,
}

impl Connection for FakeConnection {
    fn close(&mut self) {
        self.state
            .lock()
            .open
            .retain(|(_, receive)| !Arc::ptr_eq(receive, &self.receive));
    }
}
