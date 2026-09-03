//! One playing video: the decode thread's queue, the sound, the clock and
//! the frame on screen. `VideoView` owns one and asks it once per frame.

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel, sync_channel},
    },
    time::Duration,
};

use ffmpeg_next::Error as FfmpegError;
use kira::{
    Decibels, Tween,
    sound::{
        PlaybackPosition, PlaybackState, Region,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
    track::{TrackBuilder, TrackHandle},
};
use log::error;
use web_time::Instant;

use crate::{
    audio::manager::audio_manager,
    deps::refs::Weak,
    gm::{Clock, flat::Size},
    video::{
        audio::AudioDecoder,
        count_to_f64,
        decoder::{self, Command, MediaInfo, Message, VideoFrame},
        nv12::Nv12Target,
    },
    window::image::Image,
};

/// Playback counters for an overlay or a log line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VideoStats {
    /// Frames the decoder produced.
    pub decoded:              u64,
    /// Frames that reached the screen.
    pub presented:            u64,
    /// Frames skipped because a newer one was already due.
    pub dropped:              u64,
    /// Presented frames per second over the last second.
    pub presented_per_second: f64,
    /// The last frame came from the hardware decoder.
    pub hardware:             bool,
    /// The ffmpeg codec name.
    pub decoder:              String,
    pub width:                u32,
    pub height:               u32,
    /// The stream's own frame rate.
    pub frame_rate:           f64,
}

pub(crate) enum PlayerEvent {
    Finished,
    Error(String),
}

/// Sound effects play on the engine's main track, which sits at minus 20 dB.
/// A video track lifts its own sound back to unity.
const MAIN_TRACK_OFFSET: f32 = 20.0;

/// How long a stepped test waits for the decoder before giving up on a frame.
const STEPPED_WAIT: Duration = Duration::from_secs(5);

/// What reached the screen, for `VideoStats`.
struct Counters {
    presented:   u64,
    dropped:     u64,
    /// The last frame came from the hardware decoder.
    hardware:    bool,
    /// When the rate window opened, the presented count then, the last rate.
    rate_window: (Instant, u64, f64),
}

impl Default for Counters {
    fn default() -> Self {
        Self {
            presented:   0,
            dropped:     0,
            hardware:    false,
            rate_window: (Instant::now(), 0, 0.0),
        }
    }
}

/// Where the frame queue stands against the stream.
#[derive(Default)]
struct Queue {
    /// The decoder reached the end of the current generation.
    eof:          bool,
    /// A seek while paused shows its target frame once it arrives.
    seek_pending: bool,
    /// At least one frame reached the screen.
    shown:        bool,
}

struct Info {
    duration:   f64,
    frame_rate: f64,
    decoder:    String,
}

pub(crate) struct Player {
    source:     String,
    key:        String,
    commands:   Sender<Command>,
    messages:   Receiver<Message>,
    info:       Option<Info>,
    /// The sound decoder until the first play makes a sound of it.
    audio:      Option<AudioDecoder>,
    track:      Option<TrackHandle>,
    sound:      Option<StreamingSoundHandle<FfmpegError>>,
    target:     Option<Nv12Target>,
    pending:    VecDeque<VideoFrame>,
    generation: u32,
    queue:      Queue,
    failed:     bool,
    playing:    bool,
    looping:    bool,
    /// Seconds into the stream while paused, and what the clock counts from
    /// while playing without sound.
    base:       f64,
    /// `Clock` milliseconds when play started, for the clock without sound.
    started_ms: f64,
    volume:     f32,
    decoded:    Arc<AtomicU64>,
    counters:   Counters,
}

impl Player {
    pub(crate) fn open(source: &str, key: String) -> Self {
        let (commands, command_receiver) = channel();
        let (message_sender, messages) = sync_channel(decoder::QUEUE);
        let decoded = Arc::new(AtomicU64::new(0));
        decoder::spawn(
            source.to_string(),
            command_receiver,
            message_sender,
            Arc::clone(&decoded),
        );

        Self {
            source: source.to_string(),
            key,
            commands,
            messages,
            info: None,
            audio: None,
            track: None,
            sound: None,
            target: None,
            pending: VecDeque::new(),
            generation: 0,
            queue: Queue::default(),
            failed: false,
            playing: false,
            looping: false,
            base: 0.0,
            started_ms: 0.0,
            volume: 1.0,
            decoded,
            counters: Counters::default(),
        }
    }

    pub(crate) fn is_loaded(&self) -> bool {
        self.info.is_some()
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(crate) fn duration(&self) -> f64 {
        self.info.as_ref().map_or(0.0, |info| info.duration)
    }

    /// The loop keeps rendering while this is true.
    pub(crate) fn needs_frames(&self) -> bool {
        !self.failed && (self.playing || self.queue.seek_pending || !self.queue.shown)
    }

    /// The sound's position while it plays, it is the clock.
    fn sound_position(&self) -> Option<f64> {
        self.sound
            .as_ref()
            .filter(|sound| sound.state() != PlaybackState::Stopped)
            .map(StreamingSoundHandle::position)
    }

    pub(crate) fn position(&self) -> f64 {
        if let Some(seconds) = self.sound_position() {
            return seconds;
        }
        if self.playing {
            self.base + (Clock::now_ms() - self.started_ms) / 1000.0
        } else {
            self.base
        }
    }

    pub(crate) fn play(&mut self) {
        if self.playing || self.failed {
            return;
        }
        if self.queue.eof && self.pending.is_empty() && self.position() >= self.duration() {
            self.seek_to(0.0);
        }
        self.playing = true;
        self.started_ms = Clock::now_ms();
        self.start_sound();
    }

    pub(crate) fn pause(&mut self) {
        if !self.playing {
            return;
        }
        self.base = self.position();
        self.playing = false;
        if let Some(sound) = &mut self.sound {
            sound.pause(Tween::default());
        }
    }

    pub(crate) fn seek_to(&mut self, seconds: f64) {
        let duration = self.duration();
        let seconds = if duration > 0.0 {
            seconds.clamp(0.0, duration)
        } else {
            seconds.max(0.0)
        };
        self.generation += 1;
        self.pending.clear();
        self.queue.eof = false;
        self.base = seconds;
        self.started_ms = Clock::now_ms();
        self.queue.seek_pending = true;
        if self
            .commands
            .send(Command::Seek {
                generation: self.generation,
                seconds,
            })
            .is_err()
        {
            error!("video {}: the decoder is gone", self.source);
        }
        if self.sound_position().is_some() {
            if let Some(sound) = &mut self.sound {
                sound.seek_to(seconds);
            }
        } else {
            self.reset_sound();
        }
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(track) = &mut self.track {
            track.set_volume(
                Decibels(MAIN_TRACK_OFFSET + decibels(self.volume)),
                Tween::default(),
            );
        }
    }

    pub(crate) fn set_loop(&mut self, looping: bool) {
        self.looping = looping;
        if let Some(sound) = &mut self.sound {
            sound.set_loop_region(looping.then(|| Region::from(..)));
        }
    }

    pub(crate) fn stats(&mut self) -> VideoStats {
        let now = Instant::now();
        let (since, count_then, mut rate) = self.counters.rate_window;
        let elapsed = now.duration_since(since).as_secs_f64();
        if elapsed >= 1.0 {
            rate = count_to_f64(self.counters.presented - count_then) / elapsed;
            self.counters.rate_window = (now, self.counters.presented, rate);
        }
        let size = self.target.as_ref().map_or(Size::default(), Nv12Target::size);
        VideoStats {
            decoded:              self.decoded.load(Ordering::Relaxed),
            presented:            self.counters.presented,
            dropped:              self.counters.dropped,
            presented_per_second: rate,
            hardware:             self.counters.hardware,
            decoder:              self.info.as_ref().map(|info| info.decoder.clone()).unwrap_or_default(),
            width:                size.width,
            height:               size.height,
            frame_rate:           self.info.as_ref().map_or(0.0, |info| info.frame_rate),
        }
    }

    /// The image to show, when it changed, and what happened since last time.
    pub(crate) fn update(&mut self) -> (Option<Weak<Image>>, Vec<PlayerEvent>) {
        let mut events = Vec::new();
        self.receive(&mut events);
        if Clock::is_stepped() {
            self.wait_stepped(&mut events);
        }
        let image = self.present();
        self.finish(&mut events);
        (image, events)
    }

    fn receive(&mut self, events: &mut Vec<PlayerEvent>) {
        while self.pending.len() < decoder::QUEUE {
            match self.messages.try_recv() {
                Ok(message) => self.handle(message, events),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn handle(&mut self, message: Message, events: &mut Vec<PlayerEvent>) {
        match message {
            Message::Info(MediaInfo {
                duration,
                width,
                height,
                frame_rate,
                decoder,
                audio,
            }) => {
                self.audio = audio;
                self.info = Some(Info {
                    duration,
                    frame_rate,
                    decoder,
                });
                self.target = Some(Nv12Target::new(&self.key, Size::new(width, height)));
            }
            Message::Frame(frame) => {
                if frame.generation == self.generation {
                    self.pending.push_back(frame);
                }
            }
            Message::Eof { generation } => {
                if generation == self.generation {
                    self.queue.eof = true;
                }
            }
            Message::Error(message) => {
                self.failed = true;
                self.playing = false;
                events.push(PlayerEvent::Error(message));
            }
        }
    }

    /// Under stepped time the test drives the frames, so the next picture has
    /// to be in hand before the frame it is due on renders. Real time never
    /// waits, a late decoder means a dropped frame there.
    fn wait_stepped(&mut self, events: &mut Vec<PlayerEvent>) {
        let deadline = Instant::now() + STEPPED_WAIT;
        while !self.failed
            && (self.info.is_none() || (self.pending.is_empty() && !self.queue.eof && self.needs_frames()))
        {
            if Instant::now() > deadline {
                break;
            }
            match self.messages.recv_timeout(Duration::from_millis(100)) {
                Ok(message) => self.handle(message, events),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn present(&mut self) -> Option<Weak<Image>> {
        let front = self.pending.front()?.pts;
        let now = self.position();
        let slack = self.info.as_ref().map_or(0.0, |info| 0.5 / info.frame_rate);
        let show = if self.playing {
            front <= now + slack
        } else {
            !self.queue.shown || self.queue.seek_pending
        };
        if !show {
            return None;
        }

        // Late frames make way for the newest due one, they count as dropped.
        while self.playing && self.pending.len() > 1 && self.pending[1].pts <= now + slack {
            self.pending.pop_front();
            self.counters.dropped += 1;
        }
        let frame = self.pending.pop_front()?;

        let size = Size::new(frame.width, frame.height);
        if self.target.as_ref().is_none_or(|target| target.size() != size) {
            self.target = Some(Nv12Target::new(&self.key, size));
        }
        let target = self.target.as_ref()?;
        target.show(&frame);

        self.counters.hardware = frame.hardware;
        self.counters.presented += 1;
        self.queue.shown = true;
        self.queue.seek_pending = false;
        Some(target.image())
    }

    fn finish(&mut self, events: &mut Vec<PlayerEvent>) {
        if !self.playing || !self.queue.eof || !self.pending.is_empty() {
            return;
        }
        let sound_done = self.sound.as_ref().is_some_and(|sound| sound.state() == PlaybackState::Stopped);
        if !sound_done && self.position() + 0.001 < self.duration() {
            return;
        }
        if self.looping {
            self.seek_to(0.0);
            return;
        }
        self.base = self.duration();
        self.playing = false;
        if let Some(sound) = &mut self.sound {
            sound.pause(Tween::default());
        }
        events.push(PlayerEvent::Finished);
    }

    /// Makes the sound on the first play, resumes it after a pause.
    fn start_sound(&mut self) {
        if self.sound_position().is_some() {
            if let Some(sound) = &mut self.sound {
                sound.resume(Tween::default());
            }
            return;
        }
        let Some(audio) = self.audio.take() else {
            return;
        };

        let track = audio_manager()
            .add_sub_track(TrackBuilder::new().volume(Decibels(MAIN_TRACK_OFFSET + decibels(self.volume))));
        let mut track = match track {
            Ok(track) => track,
            Err(err) => {
                error!("video {}: no sound track, {err}", self.source);
                return;
            }
        };

        let mut data =
            StreamingSoundData::from_decoder(audio).start_position(PlaybackPosition::Seconds(self.base));
        if self.looping {
            data = data.loop_region(..);
        }
        match track.play(data) {
            Ok(sound) => self.sound = Some(sound),
            Err(err) => error!("video {}: no sound, {err:?}", self.source),
        }
        self.track = Some(track);
    }

    /// A stopped kira sound is gone for good, so a replay or a seek past the
    /// end needs a fresh decoder for the next play.
    fn reset_sound(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
        self.sound = None;
        if self.audio.is_none() {
            self.audio = match AudioDecoder::open(&self.source) {
                Ok(audio) => audio,
                Err(err) => {
                    error!("video {}: reopening the sound, {err}", self.source);
                    None
                }
            };
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
        if self.commands.send(Command::Stop).is_err() {
            // The decoder already left on its own.
        }
    }
}

/// Linear volume to kira's decibels, silence at zero.
fn decibels(volume: f32) -> f32 {
    if volume <= 0.0 {
        Decibels::SILENCE.0
    } else {
        20.0 * volume.log10()
    }
}
