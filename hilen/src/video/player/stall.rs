//! A seek while the stream is stalled. The picture thread takes a seek only
//! between packets and kira takes one only between reads, so a read that
//! waits on the network would hold the seek for as long as it waits.

use std::{
    sync::mpsc::{Receiver, TryRecvError, channel},
    thread::Builder,
    time::Duration,
};

use log::{debug, error};
use web_time::Instant;

use crate::{
    gm::Clock,
    video::{audio::AudioDecoder, player::Player, source::Interrupt},
};

/// How long a demuxer may sit in one read before a seek breaks it. A read
/// of one packet takes milliseconds on a network that works.
const STALLED_READ: Duration = Duration::from_millis(500);

/// How near to its seek target the position of the sound has to be to count
/// as arrived. It plays on from the target, so a frame later it is a little
/// past it.
const SEEK_ARRIVED: f64 = 0.5;

/// How long after a seek the reads are watched. A read that stalls later
/// than this is plain buffering, nothing waits behind it.
const SEEK_WATCH: Duration = Duration::from_secs(5);

/// The reads a seek may have to break, see `Player::break_stalled_reads`.
pub(super) struct SeekWatch {
    until:   Instant,
    /// The read of the picture was broken for this seek.
    picture: bool,
    /// The sound was replaced for this seek.
    sound:   bool,
}

/// The sound of a video and what opens it.
#[derive(Default)]
pub(super) struct SoundReads {
    /// Seconds the sound was asked to seek to and has not reached yet.
    /// kira takes a seek on its own thread, until then its position is
    /// the place from before the seek and must not drive the picture.
    pub(super) seeking: Option<f64>,
    /// The interrupt of the decoder inside the kira sound that plays.
    pub(super) playing: Option<Interrupt>,
    /// A fresh sound decoder opens on its thread, after a stalled one was
    /// dropped.
    pub(super) opening: Option<Receiver<Result<Option<AudioDecoder>, String>>>,
}

impl SeekWatch {
    pub(super) fn new() -> Self {
        Self {
            until:   Instant::now() + SEEK_WATCH,
            picture: false,
            sound:   false,
        }
    }
}

impl Player {
    /// The sound took the seek once its position is at the target, from
    /// then on it is the clock again.
    pub(super) fn settle_sound_seek(&mut self) {
        let Some(target) = self.sound_reads.seeking else {
            return;
        };
        let arrived = self
            .sound
            .as_ref()
            .is_none_or(|sound| (sound.position() * self.speed - target).abs() < SEEK_ARRIVED);
        if arrived {
            self.sound_reads.seeking = None;
        }
    }

    /// Breaks the reads a fresh seek waits behind. A read that has lasted
    /// longer than `STALLED_READ` is taken as stalled.
    pub(super) fn break_stalled_reads(&mut self) {
        let Some(mut watch) = self.seek_watch.take() else {
            return;
        };

        if !watch.picture && self.reads.read_time() > STALLED_READ {
            debug!(
                "video {}: the seek breaks the stalled read of the picture",
                self.source.location()
            );
            // The picture thread opens the source again and takes the seek.
            self.reads.break_read();
            watch.picture = true;
        }

        let sound_stalled = self
            .sound_reads
            .playing
            .as_ref()
            .is_some_and(|reads| reads.read_time() > STALLED_READ);
        if !watch.sound && sound_stalled {
            debug!(
                "video {}: the seek replaces the sound, its read is stalled",
                self.source.location()
            );
            self.replace_stalled_sound();
            watch.sound = true;
        }

        if Instant::now() < watch.until && !(watch.picture && watch.sound) {
            self.seek_watch = Some(watch);
        }
    }

    /// Drops the sound whose decoder waits in a read and opens a fresh one
    /// on a thread of its own, the open talks to the same slow network.
    /// Until it is there the video follows the engine clock.
    fn replace_stalled_sound(&mut self) {
        if let Some(reads) = &self.sound_reads.playing {
            // Its thread at kira ends with this, the decoder is not used
            // again.
            reads.break_read();
        }
        self.drop_sound();
        self.audio = None;

        let (send, receive) = channel();
        self.sound_reads.opening = Some(receive);
        let source = self.source.clone();
        let reads = Interrupt::new(&self.stop);
        let (track, speed) = (self.audio_track, self.speed);
        let spawned = Builder::new().name("hilen-video-sound".into()).spawn(move || {
            let opened = AudioDecoder::open(&source, reads, track, speed).map_err(|err| err.to_string());
            if send.send(opened).is_err() {
                debug!(
                    "video {}: the player no longer waits for the sound",
                    source.location()
                );
            }
        });
        if let Err(err) = spawned {
            error!(
                "video {}: no thread to open the sound, {err}",
                self.source.location()
            );
            self.sound_reads.opening = None;
        }
    }

    /// The sound decoder that opened on its thread, played from where the
    /// video is now.
    pub(super) fn take_opened_sound(&mut self) {
        let Some(opening) = &self.sound_reads.opening else {
            return;
        };
        let opened = match opening.try_recv() {
            Ok(opened) => opened,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("the thread that opens it is gone".to_string()),
        };
        self.sound_reads.opening = None;
        match opened {
            Ok(audio) => self.audio = audio,
            Err(err) => {
                error!("video {}: reopening the sound, {err}", self.source.location());
                return;
            }
        }
        if self.playing && !self.flow.buffering && self.sound.is_none() {
            self.base = self.position();
            self.started_ms = Clock::now_ms();
            self.start_sound();
        }
    }
}
