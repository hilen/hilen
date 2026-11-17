//! What a player does for a list of pieces, see `source/pieces.rs`. The
//! picture needs nothing here, the decode thread of a list hands over frames
//! like the one of a file. The sound does: it is 1 decoder over all pieces,
//! made on the spot since making it opens no file, so it never comes in
//! later from a thread the way the sound of a file does.

use log::error;

use crate::video::{VideoSource, audio::pieces::PiecesSound, player::Player, source::Interrupt};

impl Player {
    /// The source is a list of pieces.
    pub(crate) fn plays_pieces(&self) -> bool {
        self.source.piece_list().is_some()
    }

    /// Takes the gains of another list when it has the same pieces with
    /// only other gains, and says whether it did. The list plays on with
    /// the new volume, nothing is opened again.
    pub(crate) fn takes_gains_of(&self, source: &VideoSource) -> bool {
        match (self.source.piece_list(), source.piece_list()) {
            (Some(mine), Some(theirs)) => mine.take_gains(theirs),
            _ => false,
        }
    }

    /// A player of another list in place of this one, for a list that
    /// changed after a trim or a reorder. It stands at the same position.
    /// The picture of this player stays on screen until the new one has its
    /// frame, in the same image, so the state never goes through loading.
    pub(crate) fn replaced_by(&self, source: VideoSource) -> Self {
        let end = source.piece_list().map_or(0.0, |list| list.duration());
        let position = self.position().min(end);
        let mut player = Self::open(source, self.key.clone());
        player.queue.shown = self.queue.shown;
        player.seek_to(position);
        player
    }

    /// A fresh sound decoder over the whole list at the speed of the
    /// player. None while the decode thread has not looked at the files yet,
    /// and for a list with no sound track at all, which then follows the
    /// engine clock.
    pub(super) fn pieces_sound(&self) -> Option<PiecesSound> {
        let list = self.source.piece_list()?;
        if list.has_sound() != Some(true) {
            return None;
        }
        match PiecesSound::new(list, self.speed, Interrupt::new(&self.stop)) {
            Ok(sound) => Some(sound),
            Err(err) => {
                error!("video {}: no sound, {err}", self.source.location());
                None
            }
        }
    }

    /// The sound of a list was dropped, by a seek after its end or by a
    /// change of the speed. A list that plays gets its new sound here.
    pub(super) fn restart_pieces_sound(&mut self) {
        if self.playing && !self.flow.buffering && self.plays_pieces() {
            self.start_sound();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };

    use crate::video::{
        VideoMusic, VideoPiece, VideoPieces, VideoSource,
        decoder::{Matrix, Message, Transfer, VideoFrame},
        player::{Player, Upload},
        test_fixture, test_wav,
    };

    /// A player of these pieces, once its decode thread has looked at the
    /// files.
    fn player(pieces: impl IntoIterator<Item = VideoPiece>) -> Player {
        let player = Player::open(VideoSource::from_pieces(pieces), "pieces".to_string());
        let list = player.source.piece_list().expect("a list of pieces");
        let deadline = Instant::now() + Duration::from_secs(10);
        while list.has_sound().is_none() {
            assert!(
                Instant::now() < deadline,
                "the decode thread never looked at the files"
            );
            sleep(Duration::from_millis(5));
        }
        player
    }

    /// A list gets a sound when at least 1 of its files has a sound track,
    /// wherever in the list that file is. A list with no sound track gets
    /// none and follows the engine clock, a silent sound would be a clock a
    /// stepped test cannot drive.
    #[test]
    fn a_list_has_a_sound_when_a_file_has_one() {
        let silent = || VideoPiece::new(test_fixture("colors.mp4"), 0.0, 1.0);
        let sounding = || VideoPiece::new(test_fixture("ramp.mp4"), 0.0, 1.0);

        assert!(player([sounding()]).pieces_sound().is_some());
        assert!(player([silent(), silent(), sounding()]).pieces_sound().is_some());
        assert!(player([silent()]).pieces_sound().is_none());
        assert!(player([silent(), test_silent_ramp()]).pieces_sound().is_none());
    }

    fn test_silent_ramp() -> VideoPiece {
        VideoPiece::new(test_fixture("ramp_silent.mp4"), 0.0, 1.0)
    }

    /// A list with music has a sound at once, also when no file of its
    /// pieces has a sound track, so the music is heard under silent
    /// pictures. A list with only another music volume is taken by the
    /// player that plays, a list with no music is not.
    #[test]
    fn a_list_with_music_has_a_sound_and_takes_a_new_volume() {
        let music = VideoMusic::new(test_wav("player", &[8_192; 4_800]));
        let list = |volume: f32| {
            let music = music.clone().with_volume(volume);
            VideoSource::from_pieces(VideoPieces::new([test_silent_ramp()]).with_music(music))
        };
        let player = Player::open(list(1.0), "music".to_string());
        assert!(player.pieces_sound().is_some());

        assert!(player.takes_gains_of(&list(0.5)));
        let playing = player.source.piece_list().expect("a list of pieces");
        let volume = playing.music().expect("the list has music").volume();
        assert!((volume - 0.5).abs() < f32::EPSILON, "the volume is {volume}");

        assert!(!player.takes_gains_of(&VideoSource::from_pieces([test_silent_ramp()])));
    }

    /// The player of a changed list stands where the old one stood, or at
    /// the end of the new list when that is shorter.
    #[test]
    fn a_new_list_keeps_the_position() {
        let ramp = |end: f64| VideoPiece::new(test_fixture("ramp.mp4"), 0.0, end);
        let mut old = player([ramp(1.0), ramp(1.0)]);
        old.seek_to(1.5);

        let same = old.replaced_by(VideoSource::from_pieces([ramp(2.0)]));
        assert!((same.position() - 1.5).abs() < 1e-9, "{}", same.position());
        let shorter = old.replaced_by(VideoSource::from_pieces([ramp(0.5)]));
        assert!((shorter.position() - 0.5).abs() < 1e-9, "{}", shorter.position());
    }

    /// A frame of the decode thread, of no picture.
    fn frame(generation: u32, pts: f64) -> VideoFrame {
        VideoFrame {
            generation,
            pts,
            width: 2,
            height: 2,
            y: vec![16; 4],
            y_stride: 2,
            uv: vec![128; 2],
            uv_stride: 2,
            ten_bit: false,
            full_range: false,
            matrix: Matrix::Bt709,
            transfer: Transfer::Sdr,
            hardware: false,
        }
    }

    /// A drag of a slider over a paused list: a seek comes on every move,
    /// and the frame of the newest one is never there yet. The frame of an
    /// older seek shows meanwhile, the frames of the place that was left do
    /// not, and the frame of the newest seek ends the scrub.
    #[test]
    fn a_paused_scrub_shows_the_frames_of_older_seeks() {
        let mut player = player([VideoPiece::new(test_fixture("ramp.mp4"), 0.0, 2.0)]);
        player.upload = Upload::Off;
        let mut events = Vec::new();
        let left = player.generation;
        player.queue.shown = true;
        player.queue.shown_generation = left;
        player.seek_to(0.5);
        let older = player.generation;
        player.seek_to(1.0);
        let newest = player.generation;

        player.handle(Message::Frame(frame(left, 0.1)), &mut events);
        let _ = player.present();
        assert_eq!(
            player.queue.shown_generation, left,
            "a frame of the place that was left showed"
        );

        player.handle(Message::Frame(frame(older, 0.5)), &mut events);
        let _ = player.present();
        assert_eq!(player.queue.shown_generation, older);
        assert!(
            player.queue.seek_pending,
            "the newest seek still waits for its frame"
        );

        player.handle(Message::Frame(frame(newest, 1.0)), &mut events);
        let _ = player.present();
        assert_eq!(player.queue.shown_generation, newest);
        assert!(!player.queue.seek_pending);
        assert!(events.is_empty());
    }

    /// While a list plays, the frames of an older seek are dropped as
    /// before, the clock runs on and only the newest place is shown.
    #[test]
    fn a_playing_list_shows_no_frame_of_an_older_seek() {
        let mut player = player([VideoPiece::new(test_fixture("ramp.mp4"), 0.0, 2.0)]);
        player.upload = Upload::Off;
        player.seek_to(0.5);
        let older = player.generation;
        player.seek_to(1.0);
        // Set by hand, a play would start a sound, and that wants the main
        // thread.
        player.playing = true;
        let mut events = Vec::new();
        player.handle(Message::Frame(frame(older, 0.5)), &mut events);
        assert!(player.queue.scrub.is_none());
    }
}
