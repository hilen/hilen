//! Its own test binary, since the unit test in `audio/manager.rs` asserts
//! that nothing in its process opened the audio device.
#![cfg(feature = "audio")]

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use hilen::{
    audio::{Playing, Sound},
    refs::manage::ResourceLoader,
};

/// A 440 Hz tone, 1 second, Ogg Vorbis. Loaded from bytes the way an app
/// plays audio it downloaded.
const TONE: &[u8] = include_bytes!("tone.ogg");

/// A stop fades out over a few milliseconds.
const STOP_WAIT: Duration = Duration::from_millis(500);
/// The tone plus a margin for a busy machine.
const END_WAIT: Duration = Duration::from_secs(4);

/// Plays at volume 0, so a test run makes no noise. The handle behaves the
/// same at any volume.
#[test]
fn play_once_stops_early_and_ends_by_itself() {
    let mut sound = Sound::load_data(TONE, "tone.ogg");

    // A machine with no output device, like a CI runner, used to panic on
    // the first play. The sound is silent there and never reports playing.
    if !Sound::has_output() {
        assert!(!sound.play_once(0.0).is_playing());
        return;
    }

    let mut playing = sound.play_once(0.0);
    assert!(playing.is_playing());
    playing.stop();
    assert!(stops_within(&playing, STOP_WAIT), "stop did not end the tone");

    let playing = sound.play_once(0.0);
    assert!(playing.is_playing());
    assert!(stops_within(&playing, END_WAIT), "the tone never ended by itself");
}

fn stops_within(playing: &Playing, limit: Duration) -> bool {
    let started = Instant::now();
    while playing.is_playing() {
        if started.elapsed() > limit {
            return false;
        }
        sleep(Duration::from_millis(10));
    }
    true
}
