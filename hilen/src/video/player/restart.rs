//! A video that failed and plays again. The decode thread ends when it
//! gives up on its source, so a play after that starts a new one at the
//! place the video stood at.

use std::sync::{
    Arc,
    atomic::AtomicU64,
    mpsc::{Receiver, Sender, channel, sync_channel},
};

use log::{error, info};

use crate::{
    gm::flat::Size,
    video::{
        PlayerEvent, VideoSource,
        audio::AudioDecoder,
        decoder::{self, Command, MediaInfo, Message},
        nv12::Nv12Target,
        player::{Flow, Info, Player, Upload},
        source::Interrupt,
    },
};

/// Starts a decode thread over the source and returns its 2 channels. With
/// `sound` it opens the first sound decoder too.
pub(super) fn start_decoder(
    source: &VideoSource,
    decoded: &Arc<AtomicU64>,
    reads: &Interrupt,
    sound: bool,
) -> (Sender<Command>, Receiver<Message>) {
    let (commands, command_receiver) = channel();
    let (message_sender, messages) = sync_channel(decoder::QUEUE);
    decoder::spawn(
        source.clone(),
        command_receiver,
        message_sender,
        Arc::clone(decoded),
        reads.clone(),
        sound,
    );
    (commands, messages)
}

impl Player {
    /// The source opened. After a failure it opened for the second time,
    /// what the player knows and chose by then stays as it is.
    pub(super) fn take_info(&mut self, info: MediaInfo) {
        if self.info.is_some() {
            info!(
                "video {}: the source opened again after a failure",
                self.source.location()
            );
            return;
        }
        let MediaInfo {
            duration,
            width,
            height,
            frame_rate,
            decoder,
            audio,
            tracks,
        } = info;
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
        if self.upload == Upload::Gpu {
            self.target = Some(Nv12Target::new(&self.key, Size::new(width, height), false));
        }
    }

    /// The decode thread gave up and is gone. The video stops where it is,
    /// until a play starts it again.
    pub(super) fn fail(&mut self, message: String, events: &mut Vec<PlayerEvent>) {
        // Before the sound goes, while it is the clock it holds the place.
        self.base = self.position();
        error!(
            "video {}: failed at {:.2} s, {message}",
            self.source.location(),
            self.base
        );
        self.failed = true;
        self.playing = false;
        self.flow.buffering = false;
        self.flow.stand = None;
        self.pending.clear();
        // Its connection may be as dead as the one of the picture, a play
        // opens a fresh sound.
        self.drop_sound();
        events.push(PlayerEvent::Error(message));
    }

    /// Clears the failure and opens the source again at the place the video
    /// stood at, with the tracks and the speed it had.
    pub(super) fn restart(&mut self) {
        info!(
            "video {}: a play after a failure, the source opens again at {:.2} s",
            self.source.location(),
            self.base
        );
        self.failed = false;
        self.reads = Interrupt::new(&self.stop);
        // Only the first open hands over a sound decoder. Later the player
        // opens the track and the speed it plays by now.
        let first = self.info.is_none();
        let (commands, messages) = start_decoder(&self.source, &self.decoded, &self.reads, first);
        self.commands = commands;
        self.messages = messages;

        self.generation += 1;
        self.pending.clear();
        self.seek_watch = None;
        self.queue.eof = false;
        self.queue.seek_pending = true;
        self.flow = Flow {
            due: self.base,
            ..Flow::default()
        };

        // The new thread takes both before its first read.
        if let Some(track) = self.subtitles.track() {
            self.send(Command::Subtitle(Some(track)));
        }
        self.subtitles.seek(&self.source, &self.stop, self.base);
        self.send(Command::Seek {
            generation: self.generation,
            seconds:    self.base,
        });
        if !first {
            self.reopen_sound();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::atomic::Ordering,
        thread::sleep,
        time::{Duration, Instant},
    };

    use kira::sound::PlaybackState;
    use serial_test::serial;

    use crate::{
        audio::manager::audio_manager,
        deps::hreads::set_current_thread_as_main,
        video::{
            PlayerEvent, VideoSource, VideoState,
            player::{Player, Upload},
            test_server::{Cut, Mode, Server},
        },
    };

    /// The byte of `stalled.mkv` the frame at 2.5 seconds starts on. The
    /// file is 4 seconds long, with the picture and 1 sound track, and a
    /// cluster of matroska every second.
    const CUT_AT: usize = 61490;

    /// Seconds of the picture that lie before the cut.
    const CUT_SECONDS: f64 = 2.5;

    /// The pictures of `stalled.mkv`.
    const FRAMES: u64 = 120;

    /// A cut in the middle of the block of that frame, 593 bytes long, with
    /// a stop before it that is in the same block.
    const INSIDE_THE_BLOCK: Cut = Cut {
        at:   CUT_AT + 400,
        hold: Some(CUT_AT + 200),
        skip: 0,
    };

    /// How long a broken stream is tried again in these tests.
    const LIMIT: Duration = Duration::from_millis(1500);

    fn cut_server() -> Server {
        let cut = Cut {
            at:   CUT_AT,
            hold: None,
            skip: 0,
        };
        Server::start("stalled.mkv", Mode::Cut, cut)
    }

    /// A player run the way a view runs it, with every state and error it
    /// went through.
    struct Run {
        player: Player,
        states: Vec<VideoState>,
        errors: Vec<String>,
    }

    impl Run {
        fn new(source: VideoSource) -> Self {
            // The player is made for the main thread, the audio session of a
            // phone checks it. These tests run one at a time for that.
            set_current_thread_as_main();
            let mut player = Player::open(source, "restart".to_string());
            player.upload = Upload::Off;
            player.set_volume(0.0);
            Self {
                player,
                states: Vec::new(),
                errors: Vec::new(),
            }
        }

        fn note_state(&mut self) {
            let state = self.player.state();
            if self.states.last() != Some(&state) {
                self.states.push(state);
            }
        }

        fn play(&mut self) {
            self.player.play();
            self.note_state();
        }

        /// One frame of the app.
        fn frame(&mut self) {
            let (_, events) = self.player.update();
            for event in events {
                if let PlayerEvent::Error(message) = event {
                    self.errors.push(message);
                }
            }
            self.note_state();
        }

        /// Runs frames until `done` holds.
        fn until(&mut self, what: &str, time: Duration, done: impl Fn(&Player) -> bool) {
            let deadline = Instant::now() + time;
            loop {
                self.frame();
                if done(&self.player) {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for {what}: at {:.2} s, went through {:?}, errors {:?}",
                    self.player.position(),
                    self.states,
                    self.errors
                );
                sleep(Duration::from_millis(5));
            }
        }
    }

    /// A sound of the video plays or waits to.
    fn sounding(player: &Player) -> bool {
        player
            .sound
            .as_ref()
            .is_some_and(|sound| sound.state() != PlaybackState::Stopped)
    }

    /// A player that shows its first frame.
    fn loaded(source: VideoSource) -> Run {
        let mut run = Run::new(source);
        run.until("the first frame", Duration::from_secs(10), |player| {
            player.state() == VideoState::Paused
        });
        run
    }

    /// A player that played into the cut with a server that stays dead, and
    /// failed after the limit. Returns how long it was buffering.
    fn failed(server: &Server) -> (Run, Duration) {
        let mut run = loaded(server.source().with_reconnect_limit(LIMIT));
        server.set(Mode::Dead);
        run.play();

        run.until("the player to buffer", Duration::from_secs(10), |player| {
            player.state() == VideoState::Buffering
        });
        assert!(run.player.is_playing(), "a buffering player is playing");
        let buffering = Instant::now();

        run.until("the player to give up", Duration::from_secs(10), |player| {
            player.state() == VideoState::Failed
        });
        (run, buffering.elapsed())
    }

    /// The server closes the connection in the middle of the body, at the
    /// start of a block. The player went to `Failed` with "Input/output
    /// error" on the first read past the cut.
    #[test]
    #[serial]
    fn a_stream_cut_once_opens_again_and_plays_on() {
        let server = cut_server();
        let mut run = loaded(server.source());
        run.play();
        run.until("the video to play", Duration::from_secs(10), |player| {
            player.position() > 0.5
        });
        let heard = sounding(&run.player);
        // Every connection that is open got the cut answer, the one the
        // sound made for its first seek too. A new one gets the whole file.
        server.set(Mode::Whole);

        run.until(
            "a frame and the sound after the cut",
            Duration::from_secs(15),
            move |player| {
                player.flow.due > CUT_SECONDS + 0.8
                    && player.position() > CUT_SECONDS + 0.8
                    && sounding(player) == heard
            },
        );
        run.until("the end", Duration::from_secs(10), |player| {
            player.state() == VideoState::Finished
        });

        assert!(
            !run.states.contains(&VideoState::Failed),
            "never failed, went through {:?}",
            run.states
        );
        assert_eq!(run.errors, Vec::<String>::new());
        assert!(
            server.cuts.load(Ordering::Relaxed) > 0,
            "the server cut an answer short"
        );
    }

    /// The fault of the real run. The picture and the sound each read their
    /// own connection, the player is paused, and the server closes both in
    /// the middle of a block. The matroska demuxer then goes back to the
    /// start of that block, which asks the server again and gets a new
    /// connection, and looks for the next cluster. It reports no error, so
    /// the engine saw nothing: half a second of pictures was lost, and half
    /// a second of sound, which put the sound that much ahead of the
    /// picture for the rest of the film.
    #[test]
    #[serial]
    fn a_cut_the_demuxer_hides_is_seen_and_loses_nothing() {
        let server = Server::start("stalled.mkv", Mode::Cut, INSIDE_THE_BLOCK);
        let mut run = loaded(server.source());
        run.play();
        // Both readers have read all the server sent and wait in a read.
        run.until("the stream to stand still", Duration::from_secs(10), |player| {
            player.state() == VideoState::Buffering
        });
        run.player.pause();
        run.note_state();

        // The held connections get a little more and close. A new one
        // gets the whole file.
        server.set(Mode::Whole);
        server.release();
        run.play();

        run.until(
            "the picture and the sound to agree after the cut",
            Duration::from_secs(15),
            |player| {
                player.state() == VideoState::Playing
                    && player.position() > CUT_SECONDS + 0.8
                    && (player.position() - player.flow.due).abs() < 0.1
            },
        );
        let heard = audio_manager().is_some();
        assert_eq!(sounding(&run.player), heard, "the sound plays after the cut");
        run.until("the end", Duration::from_secs(10), |player| {
            player.state() == VideoState::Finished
        });

        assert_eq!(run.errors, Vec::<String>::new());
        assert!(
            !run.states.contains(&VideoState::Failed),
            "never failed, went through {:?}",
            run.states
        );
        assert_eq!(
            run.player.decoded.load(Ordering::Relaxed),
            FRAMES,
            "every picture came once, none was lost in the cut"
        );
        if heard {
            assert!(
                run.player.sound_opened_again(),
                "the sound was cut too, it got a fresh decoder"
            );
        }
        assert!(
            server.cuts.load(Ordering::Relaxed) >= 2,
            "the server cut the connection of the picture and of the sound"
        );
    }

    /// A server that stays dead: the player waits in `Buffering` while the
    /// stream is opened again and again, and fails once the limit is over.
    #[test]
    #[serial]
    fn a_stream_that_stays_broken_buffers_and_then_fails() {
        let server = cut_server();
        let (run, buffered) = failed(&server);

        assert_eq!(
            run.states,
            [
                VideoState::Loading,
                VideoState::Paused,
                VideoState::Playing,
                VideoState::Buffering,
                VideoState::Failed
            ]
        );
        assert_eq!(run.errors.len(), 1, "one error, got {:?}", run.errors);
        assert!(!run.player.is_playing(), "a failed player does not play");
        // The stall time of the player is part of the limit, so it buffers
        // a little shorter than the limit is long.
        assert!(
            buffered > LIMIT / 2 && buffered < LIMIT * 3,
            "it tried for about the limit, buffered for {buffered:?}"
        );
        let stood = run.player.position();
        assert!(
            (CUT_SECONDS - 0.5..CUT_SECONDS + 0.7).contains(&stood),
            "it stands at the cut, at {stood}"
        );
    }

    /// A failed player ignored every play. Now a play opens the source
    /// again: with the server still dead it fails again and says so, with
    /// the server back it plays on from the place it stood at.
    #[test]
    #[serial]
    fn a_play_after_a_failure_opens_the_stream_again() {
        let server = cut_server();
        let (mut run, _) = failed(&server);
        let stood = run.player.position();

        run.play();
        assert!(run.player.is_playing(), "a play on a failed player plays");
        assert_eq!(run.player.state(), VideoState::Buffering);
        run.until("the second failure", Duration::from_secs(10), |player| {
            player.state() == VideoState::Failed
        });
        assert_eq!(run.errors.len(), 2, "a second error, got {:?}", run.errors);
        assert!(!run.player.is_playing());
        assert!(
            (run.player.position() - stood).abs() < 0.05,
            "the place is kept over a failed play, {} against {stood}",
            run.player.position()
        );

        server.set(Mode::Whole);
        run.play();
        assert!(run.player.is_playing());
        assert_eq!(run.player.state(), VideoState::Buffering);
        // A machine with no audio output plays no sound at all.
        let output = audio_manager().is_some();
        run.until(
            "frames and the sound after the place it stood at",
            Duration::from_secs(15),
            move |player| {
                player.state() == VideoState::Playing
                    && player.flow.due > stood + 0.3
                    && sounding(player) == output
            },
        );
        assert!(
            run.player.position() >= stood,
            "it went on from {stood}, not from {}",
            run.player.position()
        );
        assert_eq!(
            run.states,
            [
                VideoState::Loading,
                VideoState::Paused,
                VideoState::Playing,
                VideoState::Buffering,
                VideoState::Failed,
                VideoState::Buffering,
                VideoState::Failed,
                VideoState::Buffering,
                VideoState::Playing
            ]
        );

        run.until("the end", Duration::from_secs(10), |player| {
            player.state() == VideoState::Finished
        });
        assert_eq!(run.errors.len(), 2, "no more errors, got {:?}", run.errors);
    }
}
