use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use log::info;
use parking_lot::Mutex;

use crate::{
    deps::hreads::on_main,
    midi::{
        MidiDevice, MidiMessage,
        backend::{Backend, Connection},
    },
    time::monotonic_seconds,
};

type Action = Box<dyn FnMut(MidiMessage) + Send>;

/// An open MIDI input device, from `Midi::open`. Dropping it closes the
/// device.
pub struct MidiInput {
    device:     MidiDevice,
    subscriber: Arc<Subscriber>,
    connection: Box<dyn Connection>,
}

impl MidiInput {
    pub(super) fn open(backend: &mut dyn Backend, device: &MidiDevice) -> Result<Self> {
        let subscriber = Arc::new(Subscriber::default());
        let receiver = subscriber.clone();
        let connection = backend.open(
            device,
            Box::new(move |bytes| {
                let time = monotonic_seconds();
                let Some(message) = MidiMessage::parse(bytes, time) else {
                    return;
                };
                let subscriber = receiver.clone();
                on_main(move || subscriber.deliver(message));
            }),
        )?;
        info!("MIDI device {} is open", device.name);
        Ok(Self {
            device: device.clone(),
            subscriber,
            connection,
        })
    }

    pub fn device(&self) -> &MidiDevice {
        &self.device
    }

    /// Calls `action` on the main thread for every note on, note off and
    /// control change of the device, in the order they arrived. A new action
    /// takes the place of the old one. A message that arrives while there is
    /// no action is dropped.
    pub fn on_message(&self, action: impl FnMut(MidiMessage) + Send + 'static) {
        *self.subscriber.action.lock() = Some(Box::new(action));
    }
}

impl Drop for MidiInput {
    fn drop(&mut self) {
        self.subscriber.closed.store(true, Ordering::Relaxed);
        self.connection.close();
        *self.subscriber.action.lock() = None;
        info!("MIDI device {} is closed", self.device.name);
    }
}

#[derive(Default)]
struct Subscriber {
    action: Mutex<Option<Action>>,
    /// A message can still wait for the main thread when the input is
    /// dropped. Its action must not run then, the view it calls is gone.
    closed: AtomicBool,
}

impl Subscriber {
    fn deliver(&self, message: MidiMessage) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        // The action is out of the lock while it runs, so it can call
        // `on_message` itself.
        let Some(mut action) = self.action.lock().take() else {
            return;
        };
        action(message);
        if !self.closed.load(Ordering::Relaxed) {
            self.action.lock().get_or_insert(action);
        }
    }
}
