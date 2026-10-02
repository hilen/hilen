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
    pub(super) seeking:   Option<f64>,
    /// The interrupt of the decoder inside the kira sound that plays.
    pub(super) playing:   Option<Interrupt>,
    /// A fresh sound decoder opens on its thread. The open reads the
    /// source, on the main thread a slow network would hold the frame loop.
    pub(super) opening:   Option<Receiver<Result<Option<AudioDecoder>, String>>>,
    /// The track a switch asked for. It is the track of the video only once
    /// it has opened, a track that fails to open leaves the old one named.
    pub(super) switching: Option<usize>,
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

    /// Drops the sound whose decoder waits in a read and opens a fresh one.
    /// Until it is there the video follows the engine clock.
    fn replace_stalled_sound(&mut self) {
        if let Some(reads) = &self.sound_reads.playing {
            // Its thread at kira ends with this, the decoder is not used
            // again.
            reads.break_read();
        }
        self.drop_sound();
        self.open_sound_on_thread();
    }

    /// Opens a sound decoder for the chosen track and speed on a thread of
    /// its own, it comes in through `take_opened_sound`. An open that is
    /// still on its way is left behind, its answer goes nowhere.
    pub(super) fn open_sound_on_thread(&mut self) {
        self.audio = None;

        let (send, receive) = channel();
        self.sound_reads.opening = Some(receive);
        let source = self.source.clone();
        let reads = Interrupt::new(&self.stop);
        let (track, speed) = (self.sound_reads.switching.or(self.audio_track), self.speed);
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
        // Failed or not, the switch is over. The old track is still the one
        // named, and it plays on when the new one did not open.
        let switching = self.sound_reads.switching.take();
        let audio = match opened {
            Ok(Some(audio)) => audio,
            Ok(None) => {
                error!("video {}: no sound track to open", self.source.location());
                return;
            }
            Err(err) => {
                error!("video {}: opening the sound, {err}", self.source.location());
                return;
            }
        };
        if switching.is_some() {
            self.audio_track = Some(audio.stream());
        }
        // A sound that still plays is the track from before a switch.
        if self.sound.is_some() {
            let position = self.position();
            self.drop_sound();
            self.base = position;
            self.started_ms = Clock::now_ms();
        }
        self.audio = Some(audio);
        if self.playing && !self.flow.buffering && self.sound.is_none() {
            self.base = self.position();
            self.started_ms = Clock::now_ms();
            self.start_sound();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::{Arc, atomic::AtomicBool, mpsc::channel},
        thread::sleep,
        time::{Duration, Instant},
    };

    use crate::video::{
        audio::AudioDecoder,
        decoder::Tracks,
        player::{Info, Player},
        source::Interrupt,
        test_fixture,
        tracks::audio_tracks,
    };

    /// The player of the fixture with 2 sound tracks, the 440 Hz tone at
    /// stream 1 named as the one that plays.
    fn player() -> Player {
        let source = test_fixture("tracks.mkv");
        let stop = Arc::new(AtomicBool::new(false));
        let input = source.open(&Interrupt::new(&stop)).expect("the fixture opens");
        let mut player = Player::open(source, "switch".to_string());
        player.audio_track = Some(1);
        player.info = Some(Info {
            duration:   4.0,
            frame_rate: 30.0,
            decoder:    String::new(),
            tracks:     Tracks {
                audio:     audio_tracks(&input),
                subtitles: Vec::new(),
            },
        });
        player
    }

    /// A switch to a track that fails to open used to name the new track at
    /// once, so `audio_track` said one track while the other one played.
    #[test]
    fn a_track_that_fails_to_open_leaves_the_old_one_named() {
        let mut player = player();
        player.set_audio_track(2);
        // The answer of the thread that opens the track is swapped for a
        // failed one.
        let (send, receive) = channel();
        player.sound_reads.opening = Some(receive);
        assert_eq!(
            player.audio_track(),
            Some(1),
            "the old track until the new one opened"
        );

        send.send(Err("no such stream".to_string()))
            .expect("the player waits for the sound");
        player.take_opened_sound();
        assert_eq!(
            player.audio_track(),
            Some(1),
            "the old track after the failed open"
        );
        assert_eq!(player.sound_reads.switching, None, "the switch is over");
    }

    /// The same switch with a track that opens: the new track is named once
    /// its decoder is in hand, not before.
    #[test]
    fn a_track_that_opens_is_named_once_it_opened() {
        let mut player = player();
        player.set_audio_track(2);
        assert_eq!(
            player.audio_track(),
            Some(1),
            "the old track while the new one opens"
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        while player.sound_reads.opening.is_some() && Instant::now() < deadline {
            player.take_opened_sound();
            sleep(Duration::from_millis(5));
        }
        assert_eq!(player.audio_track(), Some(2), "the new track once it opened");
        assert_eq!(player.audio.as_ref().map(AudioDecoder::stream), Some(2));
    }
}
