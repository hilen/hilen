use std::sync::{
    Arc,
    mpsc::{Sender, channel},
};

use kira::sound::SoundData;
use log::error;
use parking_lot::Mutex;

use crate::{
    audio::{
        Sound,
        clock_sound::{At, ClockSound, ClockSoundData, Command, Reading},
        manager::effects,
    },
    time::monotonic_seconds,
};

const DEFAULT_BPM: f64 = 120.0;

/// A clock that sounds are queued on. A queued sound starts on the exact
/// sample of its time, whatever the frame rate of the app is. The time is
/// counted in seconds and in beats, and the tempo sets how fast the beats go.
///
/// ```ignore
/// let mut clock = SoundClock::new(120.0);
/// clock.start();
/// clock.play_at_beat(&kick, 4.0, 1.0);
/// let line = clock.beats();
/// ```
///
/// The time is the one the audio thread is at. The speaker is later than
/// that by the delay of the output device, which the engine does not know.
/// With no output device the clock runs on the time of the system and plays
/// nothing. Dropping the clock stops it.
pub struct SoundClock {
    bpm:      f64,
    /// The number of the start the clock is in, 0 while stopped.
    run:      u64,
    starts:   u64,
    /// None with no output device.
    commands: Option<Sender<Command>>,
    reading:  Arc<Mutex<Reading>>,
}

impl SoundClock {
    /// A stopped clock at `bpm` beats a minute. The first clock opens the
    /// output device.
    pub fn new(bpm: f64) -> Self {
        let (clock, sound) = Self::connected(bpm);
        let Some(mut effects) = effects() else {
            return Self::silent(bpm);
        };
        match effects.play(ClockSoundData(sound)) {
            Ok(()) => clock,
            Err(err) => {
                error!("The sound clock plays nothing: {err:?}");
                Self::silent(bpm)
            }
        }
    }

    /// A clock and its half for the audio thread, not yet playing.
    pub(super) fn connected(bpm: f64) -> (Self, ClockSound) {
        let mut clock = Self::silent(bpm);
        let (commands, received) = channel();
        let sound = ClockSound::new(clock.bpm, received, clock.reading.clone());
        clock.commands = Some(commands);
        (clock, sound)
    }

    pub(super) fn silent(bpm: f64) -> Self {
        let bpm = checked_bpm(bpm).unwrap_or(DEFAULT_BPM);
        Self {
            bpm,
            run: 0,
            starts: 0,
            commands: None,
            reading: Arc::new(Mutex::new(Reading::stopped(bpm))),
        }
    }

    /// Starts the time at 0. With an output device second 0 is the first
    /// sample of the next audio buffer. A sound queued before the start
    /// waits for it. A running clock is left as it is.
    pub fn start(&mut self) {
        if self.is_running() {
            return;
        }
        self.starts += 1;
        self.run = self.starts;
        self.send(Command::Start { run: self.run });
        if self.commands.is_none() {
            *self.reading.lock() = Reading {
                run: self.run,
                ..Reading::stopped(self.bpm)
            };
        }
    }

    /// Stops the time, puts it back to 0 and drops every sound that is
    /// queued and did not start yet. A sound that already started plays to
    /// its end.
    pub fn stop(&mut self) {
        self.run = 0;
        self.send(Command::Stop);
    }

    pub fn is_running(&self) -> bool {
        self.run != 0
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Changes the tempo, also while the clock runs. The beat the clock is
    /// at stays, the beats after it come at the new tempo. With an output
    /// device the change lands at the next audio buffer.
    pub fn set_bpm(&mut self, bpm: f64) {
        let Some(bpm) = checked_bpm(bpm) else {
            return;
        };
        self.bpm = bpm;
        self.send(Command::SetBpm(bpm));
        if self.commands.is_none() {
            let now = monotonic_seconds();
            let mut reading = self.reading.lock();
            *reading = Reading {
                run: reading.run,
                time: now,
                seconds: reading.seconds_at(now),
                beats: reading.beats_at(now),
                bpm,
            };
        }
    }

    /// The seconds since the start, 0 while stopped.
    pub fn seconds(&self) -> f64 {
        self.seconds_at(monotonic_seconds())
    }

    /// The beats since the start, 0 while stopped.
    pub fn beats(&self) -> f64 {
        self.beats_at(monotonic_seconds())
    }

    /// The clock time at `time`, a value of `time::monotonic_seconds` like
    /// the time of a MIDI message. Exact for a time close to now.
    pub fn seconds_at(&self, time: f64) -> f64 {
        self.reading().map_or(0.0, |reading| reading.seconds_at(time))
    }

    /// The beat at `time`, see `seconds_at`. It counts with the tempo of
    /// now, so it is off for a time before the last tempo change.
    pub fn beats_at(&self, time: f64) -> f64 {
        self.reading().map_or(0.0, |reading| reading.beats_at(time))
    }

    /// Queues `sound` to start at `beat` at `volume`, 1 as recorded and 0
    /// silent. A beat that already passed starts at once.
    pub fn play_at_beat(&mut self, sound: &Sound, beat: f64, volume: f32) {
        self.play(sound, At::Beat(beat), beat, volume);
    }

    /// Queues `sound` to start at `seconds` on the clock, see
    /// `play_at_beat`. A tempo change does not move it.
    pub fn play_at_seconds(&mut self, sound: &Sound, seconds: f64, volume: f32) {
        self.play(sound, At::Seconds(seconds), seconds, volume);
    }

    fn play(&mut self, sound: &Sound, at: At, time: f64, volume: f32) {
        if !time.is_finite() {
            error!("A sound queued at {time} on a sound clock is dropped");
            return;
        }
        if self.commands.is_none() {
            return;
        }
        match sound.at_volume(volume).into_sound() {
            Ok((sound, handle)) => {
                drop(handle);
                self.send(Command::Play { at, sound });
            }
            Err(err) => error!("Failed to queue sound {sound:?}: {err:?}"),
        }
    }

    /// The newest reading of the run the clock is in. None while stopped,
    /// and after a start until the audio thread has taken it.
    fn reading(&self) -> Option<Reading> {
        let reading = *self.reading.lock();
        (self.run != 0 && reading.run == self.run).then_some(reading)
    }

    fn send(&mut self, command: Command) {
        let Some(commands) = &self.commands else {
            return;
        };
        if commands.send(command).is_err() {
            error!("The sound clock lost its audio output and plays nothing from now on");
            self.commands = None;
        }
    }
}

fn checked_bpm(bpm: f64) -> Option<f64> {
    if bpm.is_finite() && bpm > 0.0 {
        return Some(bpm);
    }
    error!("{bpm} is not a tempo, a sound clock needs more than 0 beats a minute");
    None
}
