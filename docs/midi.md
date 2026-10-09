# MIDI input

The `midi` feature lets an app list the MIDI input devices, hear when one is
plugged in or pulled out, open one and get its messages on the main thread. It is
off by default. The code is `hilen/src/midi`.

## The calls

```rust
use hilen::midi::{Midi, MidiEvent, MidiInput};

// In `setup` of a view. All calls are for the main thread.
let devices = Midi::devices();
Midi::devices_changed().val(self, move |devices| {
    // A device was plugged in or pulled out, `devices` is the new list.
});

if let Some(device) = devices.first() {
    let input = Midi::open(device)?;
    input.on_message(move |message| {
        if let MidiEvent::NoteOn { note, velocity } = message.event {
            let beat = clock.beats_at(message.time);
        }
    });
    self.input = Some(input);
}
```

- `Midi::devices()` gives a `MidiDevice` with an `id` and a `name` for every input
  device that is plugged in now.
- `Midi::devices_changed()` is a `UIEvent` with the new list. The engine reads the
  list again every second for it, none of the systems tells midir about a change.
- `Midi::open(device)` gives a `MidiInput`. Keep it, dropping it closes the device.
  The input of a device that was pulled out stays quiet. Open the device again when
  it is back in the list.
- `MidiInput::on_message(action)` calls `action` on the main thread for every
  message, in the order they arrived. It has 1 action, a new one takes the place of
  the old one.
- A `MidiMessage` has the `channel`, 0 to 15, the `time` and the `event`:
  `NoteOn { note, velocity }`, `NoteOff { note, velocity }` or
  `ControlChange { controller, value }`. A note on with velocity 0 is a note off.
  Every other kind of message is skipped.

## The time of a message

`message.time` is `hilen::time::monotonic_seconds()` at the moment the message
arrived. It is taken on the thread the system delivers the message on, not on the
frame that hands it to the app, so a slow frame does not move it.
`SoundClock::seconds_at(time)` and `beats_at(time)` turn it into the time of a
sound clock, see [sound-clock.md](sound-clock.md). That is how an app compares a
hit with a written note.

In a browser the system delivers on the main thread, so a busy frame there does
make the time late.

## Platforms

The devices come from [midir](https://github.com/Boddlnagg/midir): CoreMIDI on
macOS and iOS, ALSA on Linux, WinMM on Windows, the NDK MIDI calls on Android and
Web MIDI in a browser. A Linux build needs the ALSA headers, the same ones the
`audio` feature needs.

- In a browser the first call asks the user to allow MIDI. The list is empty until
  then, and `devices_changed` fires when the devices show up.
- Every call is for the main thread, and the engine checks that. On macOS it also
  matters to CoreMIDI, which tells about a new device through the run loop of the
  thread that made the first client. This was not tried with a real device yet.
- What is proven on which platform is in [roadmap.md](roadmap.md).

## Inside

`backend.rs` is the seam: a `Backend` lists and opens devices. `system.rs` is the
midir one, `fake.rs` the one of the tests. `input.rs` takes the bytes of a message
on the thread of the system, reads the time, parses with `message.rs` and sends the
result to the main thread with `on_main`. A message that still waits for the main
thread when its `MidiInput` is dropped is not handed over.

## Tests

`hilen/src/midi/midi_test.rs`, with a fake device that a test plugs in, pulls out
and sends bytes from on another thread.

```bash
far cargo test -p hilen --features midi --lib -- midi::
```

`a_message_of_a_core_midi_source_arrives` is ignored. It needs the MIDI server of a
Mac with a login session, see the roadmap.
