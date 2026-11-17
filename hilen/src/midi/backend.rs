use anyhow::Result;

use crate::midi::MidiDevice;

/// Gets the bytes of 1 MIDI message, on the thread the system delivers it
/// on.
pub(super) type Receive = Box<dyn FnMut(&[u8]) + Send>;

/// Where the devices come from: the system, or a fake one in a test.
pub(super) trait Backend {
    fn devices(&mut self) -> Vec<MidiDevice>;

    fn open(&mut self, device: &MidiDevice, receive: Receive) -> Result<Box<dyn Connection>>;
}

/// An open device.
pub(super) trait Connection {
    /// No message comes after this call.
    fn close(&mut self);
}
