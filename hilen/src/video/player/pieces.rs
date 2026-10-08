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

    use crate::video::{VideoPiece, VideoSource, player::Player, test_fixture};

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
}
