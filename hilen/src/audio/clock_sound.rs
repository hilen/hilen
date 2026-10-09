//! The half of a `SoundClock` that runs on the audio thread. It is one kira
//! sound that counts the time in samples and mixes each queued sound in from
//! the sample its time falls on. A kira clock cannot do that, a sound on it
//! starts at the first sample of a buffer of 128.

use std::{
    convert::Infallible,
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError},
    },
};

use kira::{
    Frame,
    info::Info,
    sound::{Sound, SoundData},
};
use parking_lot::Mutex;

use crate::{gm::LossyConvert, time::monotonic_seconds};

/// How much of the newest callback time goes into the time of second 0. A
/// callback comes a little early or late every time, their mean does not.
const ZERO_TIME_WEIGHT: f64 = 0.1;
/// A start time that sits on a sample is off it by a rounding error.
const SAMPLE_SLACK: f64 = 1e-6;
/// More sounds than this at once make the audio thread ask for memory.
const SOUNDS_AT_ONCE: usize = 256;
/// The buffers of kira are shorter than this, so the audio thread never
/// has to grow the mix buffer.
const MIX_FRAMES: usize = 4096;

#[derive(Clone, Copy)]
pub(super) enum At {
    Beat(f64),
    Seconds(f64),
}

pub(super) enum Command {
    Start { run: u64 },
    Stop,
    SetBpm(f64),
    Play { at: At, sound: Box<dyn Sound> },
}

/// Where the clock was at the start of the newest audio callback.
#[derive(Clone, Copy)]
pub(super) struct Reading {
    /// The number of the start this reading is from, 0 while stopped.
    pub run:     u64,
    /// `monotonic_seconds` of this reading.
    pub time:    f64,
    pub seconds: f64,
    pub beats:   f64,
    pub bpm:     f64,
}

impl Reading {
    pub fn stopped(bpm: f64) -> Self {
        Self {
            run: 0,
            time: monotonic_seconds(),
            seconds: 0.0,
            beats: 0.0,
            bpm,
        }
    }

    pub fn seconds_at(&self, time: f64) -> f64 {
        self.seconds + time - self.time
    }

    pub fn beats_at(&self, time: f64) -> f64 {
        self.beats + (time - self.time) * self.bpm / 60.0
    }
}

struct Queued {
    at:    At,
    sound: Box<dyn Sound>,
}

pub(super) struct ClockSound {
    commands:  Receiver<Command>,
    reading:   Arc<Mutex<Reading>>,
    run:       u64,
    seconds:   f64,
    beats:     f64,
    bpm:       f64,
    /// `monotonic_seconds` of second 0 of this run, evened out over the
    /// callbacks.
    zero_time: Option<f64>,
    queued:    Vec<Queued>,
    playing:   Vec<Box<dyn Sound>>,
    mix:       Vec<Frame>,
    /// The `SoundClock` is dropped.
    closed:    bool,
}

impl ClockSound {
    pub fn new(bpm: f64, commands: Receiver<Command>, reading: Arc<Mutex<Reading>>) -> Self {
        Self {
            commands,
            reading,
            run: 0,
            seconds: 0.0,
            beats: 0.0,
            bpm,
            zero_time: None,
            queued: Vec::with_capacity(SOUNDS_AT_ONCE),
            playing: Vec::with_capacity(SOUNDS_AT_ONCE),
            mix: vec![Frame::ZERO; MIX_FRAMES],
            closed: false,
        }
    }

    /// The start of one audio callback, `now` is its `monotonic_seconds`.
    pub fn begin_callback(&mut self, now: f64) {
        loop {
            match self.commands.try_recv() {
                Ok(command) => self.apply(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.closed = true;
                    break;
                }
            }
        }
        for sound in &mut self.playing {
            sound.on_start_processing();
        }
        self.publish(now);
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::Start { run } => {
                self.run = run;
                self.seconds = 0.0;
                self.beats = 0.0;
                self.zero_time = None;
            }
            Command::Stop => {
                self.run = 0;
                self.seconds = 0.0;
                self.beats = 0.0;
                self.queued.clear();
            }
            Command::SetBpm(bpm) => self.bpm = bpm,
            Command::Play { at, sound } => self.queued.push(Queued { at, sound }),
        }
    }

    fn publish(&mut self, now: f64) {
        let time = if self.run == 0 {
            now
        } else {
            let measured = now - self.seconds;
            let zero = self
                .zero_time
                .map_or(measured, |zero| zero + (measured - zero) * ZERO_TIME_WEIGHT);
            self.zero_time = Some(zero);
            zero + self.seconds
        };
        // A reader holds the lock for a copy of 5 numbers. When it does, this
        // callback skips its reading, the audio thread must never wait.
        if let Some(mut reading) = self.reading.try_lock() {
            *reading = Reading {
                run: self.run,
                time,
                seconds: self.seconds,
                beats: self.beats,
                bpm: self.bpm,
            };
        }
    }

    /// Mixes `sound` into `out` from the frame `offset` on.
    fn mix_in(
        mix: &mut Vec<Frame>,
        sound: &mut dyn Sound,
        out: &mut [Frame],
        offset: usize,
        dt: f64,
        info: &Info,
    ) {
        let out = &mut out[offset..];
        if mix.len() < out.len() {
            mix.resize(out.len(), Frame::ZERO);
        }
        let mix = &mut mix[..out.len()];
        sound.process(mix, dt, info);
        for (out, mixed) in out.iter_mut().zip(mix) {
            *out += *mixed;
        }
    }

    /// The frame of this buffer that `at` falls on, None when it is after
    /// the buffer. A time that already passed is frame 0, so a sound that
    /// was queued too late still plays.
    fn offset(&self, at: At, frames: f64, dt: f64) -> Option<usize> {
        let ahead = match at {
            At::Seconds(seconds) => (seconds - self.seconds) / dt,
            At::Beat(beat) => (beat - self.beats) / (dt * self.bpm / 60.0),
        };
        let frame = (ahead - SAMPLE_SLACK).ceil().max(0.0);
        (frame < frames).then(|| frame.lossy_convert())
    }
}

impl Sound for ClockSound {
    fn on_start_processing(&mut self) {
        self.begin_callback(monotonic_seconds());
    }

    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        out.fill(Frame::ZERO);
        for sound in &mut self.playing {
            Self::mix_in(&mut self.mix, sound.as_mut(), out, 0, dt, info);
        }
        if self.run != 0 {
            let frames = f64::from(u32::try_from(out.len()).unwrap_or(u32::MAX));
            let mut index = 0;
            while index < self.queued.len() {
                let Some(offset) = self.offset(self.queued[index].at, frames, dt) else {
                    index += 1;
                    continue;
                };
                let mut sound = self.queued.swap_remove(index).sound;
                Self::mix_in(&mut self.mix, sound.as_mut(), out, offset, dt, info);
                self.playing.push(sound);
            }
            self.seconds += frames * dt;
            self.beats += frames * dt * self.bpm / 60.0;
        }
        self.playing.retain(|sound| !sound.finished());
    }

    /// The sounds that already started play to their end after the clock is
    /// dropped.
    fn finished(&self) -> bool {
        self.closed && self.playing.is_empty()
    }
}

pub(super) struct ClockSoundData(pub ClockSound);

impl SoundData for ClockSoundData {
    type Error = Infallible;
    type Handle = ();

    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        Ok((Box::new(self.0), ()))
    }
}
