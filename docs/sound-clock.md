# Sound clock

`SoundClock` of the `audio` feature starts a sound at an exact time. A plain
`Sound::play` starts on the frame that calls it, up to 16 ms off at 60 frames a
second. A sound queued on a clock starts on the sample its time falls on, whatever
the frame rate is. A metronome or a written drum pattern needs that.

## The calls

```rust
use hilen::audio::{Sound, SoundClock};

let mut clock = SoundClock::new(120.0);
clock.start();

// Queue some beats ahead, from `update` or from a timer.
clock.play_at_beat(&kick, 4.0, 1.0);
clock.play_at_beat(&snare, 4.5, 0.8);
clock.play_at_seconds(&click, 10.0, 0.5);

// The place of the play line on screen, for the frame that is drawn.
let beat = clock.beats_at(hilen::time::frame_seconds());

clock.set_bpm(140.0);
clock.stop();
```

- `SoundClock::new(bpm)` makes a stopped clock. `kick` above is a `&Sound`, loaded
  the usual way.
- `start()` starts the time at 0. `stop()` stops it, puts it back to 0 and drops
  every queued sound that did not start yet. A sound that already started plays to
  its end. Dropping the clock stops it too.
- `play_at_beat(sound, beat, volume)` and `play_at_seconds(sound, seconds, volume)`
  queue a sound. The volume is 1 as recorded and 0 silent, like `play_with_volume`.
  A time that already passed starts at once. A sound queued before `start` waits
  for it.
- `seconds()` and `beats()` give the time now, 0 while stopped. They go on smoothly
  between 2 audio buffers, so a play line does not move in steps.
- `set_bpm(bpm)` changes the tempo, also while the clock runs. The beat the clock is
  at stays, the beats after it come at the new tempo. A sound queued at a beat moves
  with the tempo, a sound queued at seconds does not.
- `seconds_at(time)` and `beats_at(time)` turn a value of
  `hilen::time::monotonic_seconds()` into clock time. The time of a MIDI message is
  such a value, see [midi.md](midi.md). `beats_at` counts with the tempo of now, so
  it is off for a time before the last tempo change.

Queue a little ahead, not a whole song. A queued sound is a small object on the
audio thread, and 256 of them fit before that thread has to ask for memory.

Queue from a feed, not from `update`. The engine draws frames on demand and holds every
frame of a covered window, so a queue filled in `update` runs dry. `set_feed(feed)`
sets a call that the engine makes on the main thread every 0.1 seconds while the clock
runs, the first time right after `start`. A timer makes the calls, not a frame. A feed
that queues every sound of the next half second in each call never runs dry. It gets
the clock, to read `beats` and to queue. The first call comes a few ms after the
start, so queue a sound of beat 0 before `start`. A stopped or dropped clock gets no
calls, and with no running clock there is no timer. On native the timer is 1 thread
named `sound clock feed`, in a browser a task on the main thread. The code is
`hilen/src/audio/clock_feed.rs`.

```rust
clock.set_feed(move |clock| {
    let until = clock.beats() + clock.bpm() / 120.0;
    while next_beat < until {
        clock.play_at_beat(&kick, next_beat, 1.0);
        next_beat += 1.0;
    }
});
```

A hit whose time already passed starts at once. After a stall several of them sound
together, so a feed should skip what the clock is already past.

A play line reads `beats_at(time::frame_seconds())`, not `beats()`, see "The time of a
frame" in [dispatch.md](dispatch.md).

Every clock sound plays on the effects track, so `Sound::set_volume` scales it like
every other `Sound`. That volume starts at 0.1.

## What the time is

The time is the one the audio thread is at: the sample it writes now. The speaker
is later than that by the delay of the output device, which is small on a wired
output and large over Bluetooth. The engine does not know that delay. An app that
compares a hit with a written note gives the user a setting for it.

With no output device the clock still runs, on the time of the system, and plays
nothing. So a play line moves on a machine with no speaker too.

## Inside

The code is `hilen/src/audio/clock.rs`, the half the app holds, and
`clock_sound.rs`, the half on the audio thread.

A kira clock was not used. Kira renders in buffers of 128 samples, and a sound on a
kira clock starts at the first sample of the buffer its time falls in. That is up
to 2.7 ms late at 48 kHz, and the read time of a kira clock moves once per device
callback.

`ClockSound` is 1 kira sound that plays for as long as the clock lives. It counts
seconds and beats in samples. For each buffer it finds the sample a queued sound
starts on and mixes the sound in from there. The queued sound is a normal kira
static sound, so the decode, the volume and the change of sample rate are the ones
of every `Sound`. Commands reach it through a channel, read once per device
callback: start, stop, tempo and play.

At the start of every device callback the audio thread writes a reading: the clock
time and the `monotonic_seconds` it belongs to. `seconds()` adds the time since
then. A callback comes a little early or late every time, so the reading takes
only a tenth of each new difference. A callback that is 5 ms late moves the time
by 0.5 ms.

## Tests

`hilen/src/audio/clock_test.rs`. The tests call the kira sound of a clock buffer
by buffer and look at the samples, so no output device is opened.

```bash
far cargo test -p hilen --features audio --lib -- audio::
```
