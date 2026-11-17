use anyhow::{Result, anyhow};
use log::error;
use midir::{Ignore, MidiInput, MidiInputConnection};

use crate::midi::{
    MidiDevice,
    backend::{Backend, Connection, Receive},
};

const CLIENT_NAME: &str = "hilen";

/// The devices of the system through midir: CoreMIDI on macOS and iOS, ALSA
/// on Linux, WinMM on Windows, the NDK MIDI calls on Android and Web MIDI in
/// a browser.
#[derive(Default)]
pub(super) struct System {
    lister: Lister,
}

/// The midir client that lists the devices. An open device has a client of
/// its own, midir gives a client away when it connects.
#[derive(Default)]
enum Lister {
    #[default]
    NotOpened,
    Open(MidiInput),
    /// The system has no MIDI. Every list is empty then.
    Failed,
}

impl System {
    fn lister(&mut self) -> Option<&MidiInput> {
        if matches!(self.lister, Lister::NotOpened) {
            self.lister = match MidiInput::new(CLIENT_NAME) {
                Ok(input) => Lister::Open(input),
                Err(err) => {
                    error!("No MIDI on this system, the device list stays empty: {err}");
                    Lister::Failed
                }
            };
        }
        match &self.lister {
            Lister::Open(input) => Some(input),
            Lister::NotOpened | Lister::Failed => None,
        }
    }
}

impl Backend for System {
    fn devices(&mut self) -> Vec<MidiDevice> {
        let Some(lister) = self.lister() else {
            return Vec::new();
        };
        lister
            .ports()
            .iter()
            .map(|port| MidiDevice {
                id:   port.id(),
                name: lister.port_name(port).unwrap_or_else(|_| port.id()),
            })
            .collect()
    }

    fn open(&mut self, device: &MidiDevice, mut receive: Receive) -> Result<Box<dyn Connection>> {
        let mut input = MidiInput::new(CLIENT_NAME)?;
        // System exclusive data, the MIDI clock and active sensing. A drum
        // module sends the last one 3 times a second.
        input.ignore(Ignore::All);
        let port = input
            .find_port_by_id(&device.id)
            .ok_or_else(|| anyhow!("MIDI device {} is not plugged in", device.name))?;
        let connection = input
            .connect(&port, CLIENT_NAME, move |_, bytes, ()| receive(bytes), ())
            .map_err(|err| anyhow!("Failed to open MIDI device {}: {err}", device.name))?;
        Ok(Box::new(Open(Some(connection))))
    }
}

struct Open(Option<MidiInputConnection<()>>);

impl Connection for Open {
    fn close(&mut self) {
        if let Some(connection) = self.0.take() {
            drop(connection.close());
        }
    }
}
