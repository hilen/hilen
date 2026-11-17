/// One message of a MIDI input device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiMessage {
    /// When the message arrived, in `time::monotonic_seconds`. It is taken
    /// on the thread that got the message from the system, not on the frame
    /// that hands it to the app. `SoundClock::seconds_at` and `beats_at`
    /// turn it into the time of a sound clock.
    pub time:    f64,
    /// 0 to 15. A drum module sends on 9, which a manual calls channel 10.
    pub channel: u8,
    pub event:   MidiEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiEvent {
    /// `note` is 0 to 127, `velocity` 1 to 127.
    NoteOn { note: u8, velocity: u8 },
    /// A note on with velocity 0 is a note off too, as MIDI says.
    NoteOff { note: u8, velocity: u8 },
    /// `controller` and `value` are 0 to 127. A hi-hat pedal sends its
    /// place this way, on controller 4.
    ControlChange { controller: u8, value: u8 },
}

impl MidiMessage {
    /// None for every other kind of message, like a program change.
    pub(super) fn parse(bytes: &[u8], time: f64) -> Option<Self> {
        let &[status, first, second] = bytes else {
            return None;
        };
        let (first, second) = (first & 0x7F, second & 0x7F);
        let event = match status & 0xF0 {
            0x90 if second > 0 => MidiEvent::NoteOn {
                note:     first,
                velocity: second,
            },
            0x80 | 0x90 => MidiEvent::NoteOff {
                note:     first,
                velocity: second,
            },
            0xB0 => MidiEvent::ControlChange {
                controller: first,
                value:      second,
            },
            _ => return None,
        };
        Some(Self {
            time,
            channel: status & 0x0F,
            event,
        })
    }
}
