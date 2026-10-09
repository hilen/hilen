//! MIDI input devices, see `docs/midi.md`.

mod backend;
#[cfg(test)]
mod fake;
mod input;
mod message;
#[cfg(test)]
mod midi_test;
mod system;

use anyhow::Result;
use log::info;

use self::{backend::Backend, system::System};
pub use self::{
    input::MidiInput,
    message::{MidiEvent, MidiMessage},
};
use crate::{
    deps::{hreads::after, refs::main_lock::MainLock},
    ui::UIEvent,
};

/// No system tells midir when a device comes or goes, so the list is read
/// again this often.
const WATCH_SECONDS: f32 = 1.0;

static STATE: MainLock<State> = MainLock::new();
static CHANGED: UIEvent<Vec<MidiDevice>> = UIEvent::const_new();

/// A MIDI input device that is plugged in now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiDevice {
    /// The same for the same device while it stays plugged in. Whether it
    /// is the same after it was pulled out depends on the system.
    pub id:   String,
    pub name: String,
}

struct State {
    backend:  Box<dyn Backend>,
    known:    Vec<MidiDevice>,
    watching: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            backend:  Box::new(System::default()),
            known:    Vec::new(),
            watching: false,
        }
    }
}

/// The MIDI input devices of the machine. Every call is for the main thread.
pub struct Midi;

impl Midi {
    /// The devices that are plugged in now. In a browser the list is empty
    /// until the user allowed MIDI, the first call asks for it.
    pub fn devices() -> Vec<MidiDevice> {
        STATE.get_mut().backend.devices()
    }

    /// Fires on the main thread with the new list when a device is plugged
    /// in or pulled out. An open `MidiInput` of a device that was pulled out
    /// stays quiet, open the device again when it is back.
    pub fn devices_changed() -> &'static UIEvent<Vec<MidiDevice>> {
        let state = STATE.get_mut();
        if !state.watching {
            state.watching = true;
            state.known = state.backend.devices();
            watch();
        }
        &CHANGED
    }

    /// Opens `device`. Its messages come until the `MidiInput` is dropped.
    pub fn open(device: &MidiDevice) -> Result<MidiInput> {
        MidiInput::open(STATE.get_mut().backend.as_mut(), device)
    }
}

fn watch() {
    after(WATCH_SECONDS, || {
        check_devices();
        watch();
    });
}

fn check_devices() {
    let state = STATE.get_mut();
    let devices = state.backend.devices();
    if devices == state.known {
        return;
    }
    let names: Vec<_> = devices.iter().map(|device| device.name.as_str()).collect();
    info!("MIDI input devices changed: {names:?}");
    state.known.clone_from(&devices);
    CHANGED.trigger(devices);
}
