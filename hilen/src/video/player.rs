//! One playing video: the decode thread's queue, the sound, the clock and
//! the frame on screen. `VideoView` owns one and asks it once per frame.

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
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
        VideoSource,
        audio::{AudioDecoder, SPEEDS},
        count_to_f64,
        decoder::{self, Command, MediaInfo, Message, Tracks, VideoFrame},
        nv12::Nv12Target,
        subtitles::Subtitles,
        tracks::{AudioTrack, SubtitleTrack},
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

/// Where a video stands, read with `VideoView::state` and reported through
/// `VideoView::on_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VideoState {
    /// No source is set.
    #[default]
    Empty,
    /// The source is opening, no frame has reached the screen yet.
    Loading,
    Paused,
    Playing,
    /// Playing, but the stream has not delivered the next frame. The clock
    /// and the sound are held until it does.
    Buffering,
    /// Playback reached the end.
    Finished,
    /// The source could not be opened or decoded, `on_error` has the reason.
    Failed,
}

pub(crate) enum PlayerEvent {
    Finished,
    Error(String),
    /// The subtitle line to show, none clears it.
    Subtitle(Option<String>),
}

/// How long a stepped test waits for the decoder before giving up on a frame.
const STEPPED_WAIT: Duration = Duration::from_secs(5);

/// Seconds the next frame may be late before playback holds for it. A slow
/// frame or 2 drop as before, a stalled stream holds the clock.
const STALL: f64 = 0.25;

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

/// How playback moves along the stream.
#[derive(Default)]
struct Flow {
    /// Seconds the next frame is expected at.
    due:       f64,
    /// Playing, but held until the stream delivers frames again.
    buffering: bool,
    /// Playback reached the end and nothing moved it since.
    finished:  bool,
}

struct Info {
    duration:   f64,
    frame_rate: f64,
    decoder:    String,
    tracks:     Tracks,
}

pub(crate) struct Player {
    source:      VideoSource,
    key:         String,
    commands:    Sender<Command>,
    messages:    Receiver<Message>,
    info:        Option<Info>,
    /// The sound decoder until the first play makes a sound of it.
    audio:       Option<AudioDecoder>,
    track:       Option<TrackHandle>,
    sound:       Option<StreamingSoundHandle<FfmpegError>>,
    target:      Option<Nv12Target>,
    pending:     VecDeque<VideoFrame>,
    generation:  u32,
    queue:       Queue,
    flow:        Flow,
    failed:      bool,
    playing:     bool,
    looping:     bool,
    /// Seconds into the stream while paused, and what the clock counts from
    /// while playing without sound.
    base:        f64,
    /// `Clock` milliseconds when play started, for the clock without sound.
    started_ms:  f64,
    volume:      f32,
    decoded:     Arc<AtomicU64>,
    counters:    Counters,
    /// Set when the player drops, it breaks a read that waits on the network.
    stop:        Arc<AtomicBool>,
    /// The stream index of the sound track that plays.
    audio_track: Option<usize>,
    /// How fast the video plays, 1 is its own speed.
    speed:       f64,
    subtitles:   Subtitles,
}

impl Player {
    pub(crate) fn open(source: VideoSource, key: String) -> Self {
        let (commands, command_receiver) = channel();
        let (message_sender, messages) = sync_channel(decoder::QUEUE);
        let decoded = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        decoder::spawn(
            source.clone(),
            command_receiver,
            message_sender,
            Arc::clone(&decoded),
            Arc::clone(&stop),
        );

        Self {
            source,
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
            flow: Flow::default(),
            failed: false,
            playing: false,
            looping: false,
            base: 0.0,
            started_ms: 0.0,
            volume: 1.0,
            decoded,
            counters: Counters::default(),
            stop,
            audio_track: None,
            speed: 1.0,
            subtitles: Subtitles::default(),
        }
    }

    pub(crate) fn is_loaded(&self) -> bool {
        self.info.is_some()
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(crate) fn state(&self) -> VideoState {
        if self.failed {
            VideoState::Failed
        } else if !self.queue.shown {
            VideoState::Loading
        } else if self.flow.buffering {
            VideoState::Buffering
        } else if self.playing {
            VideoState::Playing
        } else if self.flow.finished {
            VideoState::Finished
        } else {
            VideoState::Paused
        }
    }

    pub(crate) fn duration(&self) -> f64 {
        self.info.as_ref().map_or(0.0, |info| info.duration)
    }

    /// The loop keeps rendering while this is true.
    pub(crate) fn needs_frames(&self) -> bool {
        !self.failed
            && (self.playing || self.queue.seek_pending || !self.queue.shown || self.subtitles.pending())
    }

    /// The sound's position while it plays, it is the clock. kira hears a
    /// sound that is shorter or longer by the speed, so its position times
    /// the speed is the position in the stream.
    fn sound_position(&self) -> Option<f64> {
        self.sound
            .as_ref()
            .filter(|sound| sound.state() != PlaybackState::Stopped)
            .map(|sound| sound.position() * self.speed)
    }

    pub(crate) fn position(&self) -> f64 {
        if let Some(seconds) = self.sound_position() {
            return seconds;
        }
        if self.playing && !self.flow.buffering {
            self.base + (Clock::now_ms() - self.started_ms) / 1000.0 * self.speed
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
        self.flow.finished = false;
        self.started_ms = Clock::now_ms();
        self.start_sound();
    }

    pub(crate) fn pause(&mut self) {
        if !self.playing {
            return;
        }
        self.base = self.position();
        self.playing = false;
        self.flow.buffering = false;
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
        self.flow.finished = false;
        self.flow.due = seconds;
        self.base = seconds;
        self.started_ms = Clock::now_ms();
        self.queue.seek_pending = true;
        self.subtitles.seek();
        if self
            .commands
            .send(Command::Seek {
                generation: self.generation,
                seconds,
            })
            .is_err()
        {
            error!("video {}: the decoder is gone", self.source.location());
        }
        if self.sound_position().is_some() {
            if let Some(sound) = &mut self.sound {
                sound.seek_to(seconds / self.speed);
            }
        } else {
            self.reset_sound();
        }
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(track) = &mut self.track {
            track.set_volume(Decibels(decibels(self.volume)), Tween::default());
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
            decoder:              self.info.as_ref().map_or_default(|info| info.decoder.clone()),
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
        self.hold_for_frames();
        let image = self.present();
        self.finish(&mut events);
        if self.subtitles.update(self.position(), &self.source) {
            events.push(PlayerEvent::Subtitle(
                self.subtitles.shown().map(ToString::to_string),
            ));
        }
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
            Message::Info(info) => {
                let MediaInfo {
                    duration,
                    width,
                    height,
                    frame_rate,
                    decoder,
                    audio,
                    tracks,
                } = *info;
                self.audio_track = audio.as_ref().map(AudioDecoder::stream);
                self.audio = audio;
                // The decode thread opened the sound at its own speed.
                if (self.speed - 1.0).abs() > f64::EPSILON {
                    self.reopen_sound();
                }
                self.info = Some(Info {
                    duration,
                    frame_rate,
                    decoder,
                    tracks,
                });
                self.target = Some(Nv12Target::new(&self.key, Size::new(width, height), false));
            }
            Message::Frame(frame) => {
                if frame.generation == self.generation {
                    self.pending.push_back(frame);
                }
            }
            Message::Cue(cue) => {
                if cue.generation == self.generation {
                    self.subtitles.push(cue);
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

    /// Holds the clock and the sound while the stream has no frame for the
    /// picture, and lets them go once the queue is full again. Real time
    /// only, a stepped test waits for the decoder instead.
    fn hold_for_frames(&mut self) {
        if !self.playing || self.failed {
            return;
        }
        let ready = self.pending.len() >= decoder::QUEUE || self.queue.eof;
        if self.flow.buffering {
            if ready || Clock::is_stepped() {
                self.flow.buffering = false;
                self.started_ms = Clock::now_ms();
                self.start_sound();
            }
            return;
        }
        if Clock::is_stepped() || ready || !self.pending.is_empty() {
            return;
        }
        // Before the first frame there is nothing to be late against, the
        // clock waits for the source to open.
        if self.queue.shown && self.position() <= self.flow.due + STALL {
            return;
        }
        self.base = self.position();
        self.flow.buffering = true;
        if let Some(sound) = &mut self.sound {
            sound.pause(Tween::default());
        }
    }

    fn present(&mut self) -> Option<Weak<Image>> {
        if self.flow.buffering {
            return None;
        }
        let front = self.pending.front()?.pts;
        let now = self.position();
        let interval = self.info.as_ref().map_or(0.0, |info| 1.0 / info.frame_rate);
        let slack = interval / 2.0;
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

        if self.target.as_ref().is_none_or(|target| !target.fits(&frame)) {
            let size = Size::new(frame.width, frame.height);
            self.target = Some(Nv12Target::new(&self.key, size, frame.ten_bit));
        }
        let target = self.target.as_ref()?;
        target.show(&frame);

        self.counters.hardware = frame.hardware;
        self.counters.presented += 1;
        self.queue.shown = true;
        self.queue.seek_pending = false;
        self.flow.due = frame.pts + interval;
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
        self.flow.finished = true;
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

        let Some(mut manager) = audio_manager() else {
            error!("video {}: no sound, no audio output", self.source.location());
            return;
        };
        let track = manager.add_sub_track(TrackBuilder::new().volume(Decibels(decibels(self.volume))));
        drop(manager);
        let mut track = match track {
            Ok(track) => track,
            Err(err) => {
                error!("video {}: no sound track, {err}", self.source.location());
                return;
            }
        };

        let mut data = StreamingSoundData::from_decoder(audio)
            .start_position(PlaybackPosition::Seconds(self.base / self.speed));
        if self.looping {
            data = data.loop_region(..);
        }
        match track.play(data) {
            Ok(sound) => self.sound = Some(sound),
            Err(err) => error!("video {}: no sound, {err:?}", self.source.location()),
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
            self.audio = match AudioDecoder::open(&self.source, &self.stop, self.audio_track, self.speed) {
                Ok(audio) => audio,
                Err(err) => {
                    error!("video {}: reopening the sound, {err}", self.source.location());
                    None
                }
            };
        }
    }
}

/// The tracks of the source: the sound track choice and the subtitles.
impl Player {
    pub(crate) fn audio_tracks(&self) -> &[AudioTrack] {
        self.info.as_ref().map_or(&[], |info| &info.tracks.audio)
    }

    pub(crate) fn subtitle_tracks(&self) -> &[SubtitleTrack] {
        self.info.as_ref().map_or(&[], |info| &info.tracks.subtitles)
    }

    pub(crate) fn audio_track(&self) -> Option<usize> {
        self.audio_track
    }

    /// Switches the sound to another track and keeps the position.
    pub(crate) fn set_audio_track(&mut self, index: usize) {
        if self.audio_track == Some(index) || !self.audio_tracks().iter().any(|track| track.index == index) {
            return;
        }
        let audio = match AudioDecoder::open(&self.source, &self.stop, Some(index), self.speed) {
            Ok(Some(audio)) => audio,
            Ok(None) => {
                error!("video {}: no sound track {index}", self.source.location());
                return;
            }
            Err(err) => {
                error!(
                    "video {}: sound track {index} did not open, {err}",
                    self.source.location()
                );
                return;
            }
        };

        let position = self.position();
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
        self.sound = None;
        self.audio = Some(audio);
        self.audio_track = Some(index);
        self.base = position;
        self.started_ms = Clock::now_ms();
        if self.playing && !self.flow.buffering {
            self.start_sound();
        }
    }

    /// Plays faster or slower from where it is. The pitch of the sound is
    /// kept.
    pub(crate) fn set_speed(&mut self, speed: f64) {
        let speed = speed.clamp(SPEEDS.0, SPEEDS.1);
        if (speed - self.speed).abs() < f64::EPSILON {
            return;
        }
        let position = self.position();
        self.speed = speed;
        self.base = position;
        self.started_ms = Clock::now_ms();
        if self.info.is_some() {
            self.reopen_sound();
        }
    }

    /// A fresh sound decoder for the current track and speed, playing on
    /// from the current position when the video plays.
    fn reopen_sound(&mut self) {
        let position = self.position();
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
        self.sound = None;
        self.base = position;
        self.started_ms = Clock::now_ms();
        if self.audio_track.is_none() {
            return;
        }
        self.audio = match AudioDecoder::open(&self.source, &self.stop, self.audio_track, self.speed) {
            Ok(audio) => audio,
            Err(err) => {
                error!("video {}: reopening the sound, {err}", self.source.location());
                None
            }
        };
        if self.playing && !self.flow.buffering {
            self.start_sound();
        }
    }

    /// Shows the cues of a subtitle track of the source, or none.
    pub(crate) fn set_subtitle_track(&mut self, index: Option<usize>) {
        self.subtitles.reset();
        self.send(Command::Subtitle(index));
        if index.is_some() {
            self.restart_picture();
        }
    }

    /// Shows the cues of a subtitle file from outside the source.
    pub(crate) fn set_subtitle_file(&mut self, source: VideoSource) {
        self.send(Command::Subtitle(None));
        self.subtitles.load_file(source, &self.stop);
    }

    pub(crate) fn subtitle(&self) -> Option<&str> {
        self.subtitles.shown()
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            error!("video {}: the decoder is gone", self.source.location());
        }
    }

    /// Decodes again from where the picture is, with the sound left alone.
    /// The demuxer runs ahead of the picture, so the cues of a track picked
    /// just now have already gone by, this brings them back.
    fn restart_picture(&mut self) {
        if self.info.is_none() {
            return;
        }
        let seconds = self.position();
        self.generation += 1;
        self.pending.clear();
        self.queue.eof = false;
        self.queue.seek_pending = true;
        self.flow.due = seconds;
        self.send(Command::Seek {
            generation: self.generation,
            seconds,
        });
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
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
