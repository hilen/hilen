//! The tests play the audio thread by hand: they call the kira sound of a
//! clock buffer by buffer and look at the samples it wrote. No output device
//! is opened.

use kira::{
    Frame,
    info::{Info, MockInfoBuilder},
    sound::Sound as KiraSound,
};

use crate::{
    audio::{Sound, SoundClock, clock_sound::ClockSound},
    time::monotonic_seconds,
};

const SAMPLE_RATE: u32 = 48_000;
const DT: f64 = 1.0 / 48_000.0;

/// What a device would have played, with the callbacks on time.
struct Output {
    sound:   ClockSound,
    info:    Info<'static>,
    samples: Vec<f32>,
    now:     f64,
}

impl Output {
    fn new(sound: ClockSound) -> Self {
        Self {
            sound,
            info: MockInfoBuilder::new().build(),
            samples: Vec::new(),
            now: 100.0,
        }
    }

    /// One device callback of `frames`, which kira cuts into buffers of
    /// `buffer`. It starts `late` seconds after its time.
    fn callback(&mut self, frames: usize, buffer: usize, late: f64) {
        self.sound.begin_callback(self.now + late);
        let mut left = frames;
        while left > 0 {
            let mut out = vec![Frame::from_mono(9.0); left.min(buffer)];
            self.sound.process(&mut out, DT, &self.info);
            self.samples.extend(out.iter().map(|frame| frame.left));
            left -= out.len();
        }
        self.now += f64::from(u32::try_from(frames).unwrap()) * DT;
    }

    fn render(&mut self, callbacks: usize, frames: usize, buffer: usize) {
        for _ in 0..callbacks {
            self.callback(frames, buffer, 0.0);
        }
    }

    /// The samples a sound starts on: not silent after a silent one.
    fn starts(&self) -> Vec<usize> {
        (0..self.samples.len())
            .filter(|&at| self.samples[at] != 0.0 && (at == 0 || self.samples[at - 1] == 0.0))
            .collect()
    }
}

fn click() -> Sound {
    Sound::from_frames(SAMPLE_RATE, vec![Frame::from_mono(1.0); 8])
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_queued_sound_starts_on_its_sample_with_any_buffer_size() {
    for (frames, buffer) in [(512, 128), (480, 128), (1024, 100), (37, 37), (4800, 128)] {
        let (mut clock, sound) = SoundClock::connected(120.0);
        let mut output = Output::new(sound);
        clock.start();
        clock.play_at_seconds(&click(), 0.01, 1.0);
        clock.play_at_beat(&click(), 0.25, 1.0);
        output.render(9600 / frames + 1, frames, buffer);
        // 0.01 seconds is sample 480, a quarter of a beat at 120 is 0.125
        // seconds, sample 6000.
        assert_eq!(
            output.starts(),
            [480, 6000],
            "{frames} frames in buffers of {buffer}"
        );
    }
}

#[test]
fn a_sound_queued_before_the_start_waits_for_it() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    clock.play_at_beat(&click(), 0.0, 1.0);
    output.render(4, 480, 128);
    clock.start();
    output.render(4, 480, 128);
    assert_eq!(output.starts(), [1920]);
}

#[test]
fn a_tempo_change_moves_the_beats_after_it() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    clock.start();
    clock.play_at_beat(&click(), 2.0, 1.0);
    clock.play_at_seconds(&click(), 0.9, 1.0);
    // Beat 1 is at half a second.
    output.render(50, 480, 128);
    assert!(close(clock.beats_at(output.now), 1.0));
    clock.set_bpm(240.0);
    output.render(50, 480, 128);
    // Beat 2 is a quarter of a second later, the sound at 0.9 seconds
    // stays there.
    assert_eq!(output.starts(), [36_000, 43_200]);
    assert!(close(clock.beats_at(output.now), 3.0));
    assert!(close(clock.seconds_at(output.now), 1.0));
}

#[test]
fn stop_drops_what_is_queued_and_the_time_starts_again_at_0() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    clock.start();
    clock.play_at_seconds(&click(), 0.5, 1.0);
    output.render(10, 480, 128);
    assert!(close(clock.seconds_at(output.now), 0.1));

    clock.stop();
    assert!(!clock.is_running());
    assert!(close(clock.seconds_at(output.now), 0.0));
    output.render(10, 480, 128);

    clock.start();
    // The audio thread has not taken the start yet.
    assert!(close(clock.seconds_at(output.now), 0.0));
    output.render(100, 480, 128);
    assert!(close(clock.seconds_at(output.now), 1.0));
    assert_eq!(output.starts(), [] as [usize; 0]);
}

#[test]
fn a_sound_queued_too_late_starts_at_once() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    clock.start();
    output.render(2, 500, 128);
    clock.play_at_seconds(&click(), 0.001, 1.0);
    output.render(2, 500, 128);
    assert_eq!(output.starts(), [1000]);
}

#[test]
fn the_time_goes_on_between_callbacks_and_a_late_callback_hardly_moves_it() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    assert!(close(clock.seconds(), 0.0));
    clock.start();
    output.render(100, 480, 128);
    assert!(close(clock.seconds_at(output.now), 1.0));
    assert!(close(clock.seconds_at(output.now + 0.004), 1.004));
    assert!(close(clock.beats_at(output.now + 0.004), 2.008));

    // This callback comes 5 ms late. The time the clock gives moves by a
    // tenth of that.
    output.callback(480, 128, 0.005);
    assert!(close(clock.seconds_at(output.now), 1.01 - 0.0005));
}

#[test]
fn the_sounds_that_started_play_on_after_the_clock_is_dropped() {
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    clock.start();
    clock.play_at_seconds(&click(), 0.0, 1.0);
    output.callback(4, 4, 0.0);
    drop(clock);
    output.callback(2, 2, 0.0);
    assert!(!output.sound.finished());
    output.render(4, 480, 128);
    assert!(output.sound.finished());
    assert_eq!(output.starts(), [0]);
    assert!(output.samples[..8].iter().all(|sample| *sample != 0.0));
}

#[test]
fn with_no_output_device_the_clock_runs_on_the_time_of_the_system() {
    let mut clock = SoundClock::silent(120.0);
    assert!(close(clock.seconds(), 0.0));
    clock.start();
    clock.play_at_beat(&click(), 1.0, 1.0);
    let now = monotonic_seconds();
    assert!(clock.seconds_at(now) >= 0.0 && clock.seconds_at(now) < 1.0);
    assert!(close(clock.seconds_at(now + 3.0) - clock.seconds_at(now), 3.0));
    assert!(close(clock.beats_at(now + 3.0) - clock.beats_at(now), 6.0));

    let before = clock.beats();
    clock.set_bpm(60.0);
    let after = clock.beats();
    assert!(after >= before && after - before < 0.5);
    let now = monotonic_seconds();
    assert!(close(clock.beats_at(now + 3.0) - clock.beats_at(now), 3.0));

    clock.stop();
    assert!(close(clock.seconds(), 0.0));
    assert!(close(clock.beats(), 0.0));
}

#[test]
fn a_wrong_tempo_is_not_taken() {
    let mut clock = SoundClock::silent(f64::NAN);
    assert!(close(clock.bpm(), 120.0));
    clock.set_bpm(0.0);
    clock.set_bpm(-4.0);
    assert!(close(clock.bpm(), 120.0));
    clock.set_bpm(280.0);
    assert!(close(clock.bpm(), 280.0));
}
