//! These tests take the place of the main thread and of the frame: each one
//! makes its own thread the main one and runs the queue of the main thread by
//! hand. They run one at a time for that.

use std::{
    sync::Arc,
    thread::{sleep, spawn},
    time::Duration,
};

use parking_lot::Mutex;
use serial_test::serial;

use crate::{
    deps::{
        hreads::{invoke_dispatched, set_current_thread_as_main},
        refs::Own,
    },
    midi::{
        CHANGED, Midi, MidiDevice, MidiEvent, MidiInput, MidiMessage, STATE, check_devices, fake::FakeMidi,
    },
    time::monotonic_seconds,
};

const SNARE_ON: [u8; 3] = [0x99, 38, 100];
const SNARE_OFF: [u8; 3] = [0x89, 38, 64];
const HI_HAT_PEDAL: [u8; 3] = [0xB9, 4, 90];

/// A fake system with a drum module plugged in.
fn drum_module() -> (FakeMidi, MidiDevice) {
    set_current_thread_as_main();
    let fake = FakeMidi::default();
    let state = STATE.get_mut();
    state.backend = Box::new(fake.clone());
    state.known = Vec::new();
    let device = fake.plug("TD-07");
    (fake, device)
}

/// Every message an input hands to the app.
fn heard(input: &MidiInput) -> Arc<Mutex<Vec<MidiMessage>>> {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let store = heard.clone();
    input.on_message(move |message| store.lock().push(message));
    heard
}

/// Sends from another thread, the way a system delivers, and returns the
/// time right before and right after.
fn send_from_a_device_thread(fake: &FakeMidi, device: &MidiDevice, messages: &[[u8; 3]]) -> (f64, f64) {
    let (fake, device, messages) = (fake.clone(), device.clone(), messages.to_vec());
    spawn(move || {
        let before = monotonic_seconds();
        for message in messages {
            fake.send(&device, &message);
        }
        (before, monotonic_seconds())
    })
    .join()
    .unwrap()
}

#[test]
fn the_3_kinds_of_message_are_read_and_the_rest_is_skipped() {
    let event = |bytes: &[u8]| MidiMessage::parse(bytes, 1.5).map(|message| (message.channel, message.event));
    assert_eq!(
        MidiMessage::parse(&SNARE_ON, 1.5),
        Some(MidiMessage {
            time:    1.5,
            channel: 9,
            event:   MidiEvent::NoteOn {
                note:     38,
                velocity: 100,
            },
        })
    );
    assert_eq!(
        event(&SNARE_OFF),
        Some((
            9,
            MidiEvent::NoteOff {
                note:     38,
                velocity: 64,
            }
        ))
    );
    assert_eq!(
        event(&[0x90, 60, 0]),
        Some((
            0,
            MidiEvent::NoteOff {
                note:     60,
                velocity: 0,
            }
        ))
    );
    assert_eq!(
        event(&HI_HAT_PEDAL),
        Some((
            9,
            MidiEvent::ControlChange {
                controller: 4,
                value:      90,
            }
        ))
    );
    // A program change, a pitch bend, active sensing and nothing at all.
    assert_eq!(event(&[0xC9, 1]), None);
    assert_eq!(event(&[0xE0, 0, 64]), None);
    assert_eq!(event(&[0xFE]), None);
    assert_eq!(event(&[]), None);
}

#[test]
#[serial]
fn a_hit_reaches_the_main_thread_with_the_time_it_arrived_at() {
    let (fake, device) = drum_module();
    assert_eq!(Midi::devices(), [device.clone()]);
    let input = Midi::open(&device).unwrap();
    assert_eq!(input.device(), &device);
    let heard = heard(&input);

    let (before, after) = send_from_a_device_thread(&fake, &device, &[SNARE_ON, SNARE_OFF, HI_HAT_PEDAL]);
    assert!(heard.lock().is_empty(), "a message waits for the main thread");

    // The frame that hands the messages over comes late.
    sleep(Duration::from_millis(40));
    invoke_dispatched();
    let delivered = monotonic_seconds();

    let heard = heard.lock().clone();
    let events: Vec<_> = heard.iter().map(|message| message.event).collect();
    assert_eq!(
        events,
        [
            MidiEvent::NoteOn {
                note:     38,
                velocity: 100,
            },
            MidiEvent::NoteOff {
                note:     38,
                velocity: 64,
            },
            MidiEvent::ControlChange {
                controller: 4,
                value:      90,
            },
        ]
    );
    for message in &heard {
        assert_eq!(message.channel, 9);
        assert!(before <= message.time && message.time <= after);
        assert!(delivered - message.time >= 0.04);
    }
    assert!(heard.is_sorted_by(|a, b| a.time <= b.time));
}

#[test]
#[serial]
fn a_dropped_input_closes_the_device_and_drops_what_still_waits() {
    let (fake, device) = drum_module();
    let input = Midi::open(&device).unwrap();
    let heard = heard(&input);
    assert_eq!(fake.open_count(), 1);

    send_from_a_device_thread(&fake, &device, &[SNARE_ON]);
    drop(input);
    assert_eq!(fake.open_count(), 0);
    invoke_dispatched();
    assert!(heard.lock().is_empty());
}

#[test]
#[serial]
fn an_action_can_set_the_next_action() {
    let (fake, device) = drum_module();
    // The input sits in an owned value, the way a view keeps it in a field.
    let input = Own::new(Midi::open(&device).unwrap());
    let first = Arc::new(Mutex::new(Vec::new()));
    let second = Arc::new(Mutex::new(Vec::new()));

    let (holder, first_store, second_store) = (input.weak(), first.clone(), second.clone());
    input.on_message(move |message| {
        first_store.lock().push(message.event);
        let second_store = second_store.clone();
        holder.on_message(move |message| second_store.lock().push(message.event));
    });

    send_from_a_device_thread(&fake, &device, &[SNARE_ON, SNARE_OFF]);
    invoke_dispatched();
    assert_eq!(first.lock().len(), 1);
    assert_eq!(second.lock().len(), 1);
}

#[test]
#[serial]
fn a_device_that_is_plugged_in_or_pulled_out_is_heard() {
    let (fake, module) = drum_module();
    STATE.get_mut().known = Midi::devices();
    let lists = Arc::new(Mutex::new(Vec::new()));
    let store = lists.clone();
    // Any owned value stands in for the view that listens.
    let listener = Own::new(1_u8);
    CHANGED
        .clear_subscribers()
        .val(listener.weak(), move |devices| store.lock().push(devices));

    check_devices();
    assert!(lists.lock().is_empty(), "nothing changed yet");

    let pad = fake.plug("SPD-SX");
    check_devices();
    check_devices();
    assert_eq!(*lists.lock(), [vec![module.clone(), pad.clone()]]);

    let input = Midi::open(&module).unwrap();
    fake.unplug(&module);
    check_devices();
    assert_eq!(*lists.lock(), [vec![module.clone(), pad.clone()], vec![pad]]);
    assert_eq!(fake.open_count(), 0);
    assert!(Midi::open(&module).is_err());
    drop(input);
    CHANGED.clear_subscribers();
}

/// The real path on a mac, with no hardware: a virtual CoreMIDI source of
/// this process is listed, opened and heard.
#[cfg(macos)]
#[test]
#[serial]
#[ignore = "needs the MIDI server of a mac, run it by name with --ignored"]
fn a_message_of_a_core_midi_source_arrives() {
    use midir::{MidiOutput, os::unix::VirtualOutput};

    use crate::midi::system::System;

    set_current_thread_as_main();
    let mut source = MidiOutput::new("hilen test")
        .unwrap()
        .create_virtual("hilen virtual drums")
        .unwrap();
    STATE.get_mut().backend = Box::new(System::default());
    let device = Midi::devices()
        .into_iter()
        .find(|device| device.name.contains("hilen virtual drums"))
        .expect("the virtual source is in the device list");
    let input = Midi::open(&device).unwrap();
    let heard = heard(&input);

    let before = monotonic_seconds();
    source.send(&SNARE_ON).unwrap();
    for _ in 0..200 {
        invoke_dispatched();
        if !heard.lock().is_empty() {
            break;
        }
        sleep(Duration::from_millis(10));
    }

    let heard = heard.lock().clone();
    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].channel, 9);
    assert_eq!(
        heard[0].event,
        MidiEvent::NoteOn {
            note:     38,
            velocity: 100,
        }
    );
    assert!(before <= heard[0].time && heard[0].time <= monotonic_seconds());
}
