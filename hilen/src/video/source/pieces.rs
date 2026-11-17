//! A list of pieces that plays as 1 video. Each piece is a source with the
//! seconds of it that play. The list travels inside a `VideoSource`, so the
//! player, its decode thread and its sound get it the way they get a file.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU32, Ordering},
};

use crate::video::{
    VideoSource,
    source::{Music, VideoPieces},
};

/// One piece of a list that plays as 1 video: a source and the part of it
/// that plays, from `start` up to `end`, in seconds of that source. The
/// frame at `end` is not part of the piece, it is the first frame of a piece
/// that starts there.
#[derive(Clone)]
pub struct VideoPiece {
    pub source:         VideoSource,
    pub start:          f64,
    pub end:            f64,
    /// What the sound of the piece is multiplied by. 1 is the sound of the
    /// source, 2 is twice as loud, 0 is silence. A sample that goes past
    /// full scale is cut flat. The picture is not changed.
    pub gain:           f32,
    /// Seconds the sound of the piece rises from silence at its start and
    /// falls to silence at its end, so a cut to another sound does not
    /// click or jump. 0 is a hard cut. The music of the list is not faded.
    pub sound_fade_in:  f64,
    pub sound_fade_out: f64,
    /// Seconds the sound of this piece and the sound of the next one
    /// overlap at the cut between them. This one plays on past its end and
    /// fades out, the next one starts before its start and fades in, half
    /// the time on each side of the cut. 0 is a hard cut. The picture is
    /// not changed, and the last piece has no cut after it.
    pub crossfade:      f64,
}

impl VideoPiece {
    pub fn new(source: impl Into<VideoSource>, start: f64, end: f64) -> Self {
        Self {
            source: source.into(),
            start,
            end,
            gain: 1.0,
            sound_fade_in: 0.0,
            sound_fade_out: 0.0,
            crossfade: 0.0,
        }
    }

    /// The same piece with its sound crossfaded into the next piece over
    /// these seconds, see `crossfade`.
    #[must_use]
    pub fn with_crossfade(mut self, seconds: f64) -> Self {
        self.crossfade = seconds.max(0.0);
        self
    }

    /// The same piece with its sound faded in at the start and out at the
    /// end, in seconds. A fade longer than half the piece is cut to half.
    #[must_use]
    pub fn with_sound_fades(mut self, fade_in: f64, fade_out: f64) -> Self {
        let half = self.duration() / 2.0;
        self.sound_fade_in = fade_in.clamp(0.0, half);
        self.sound_fade_out = fade_out.clamp(0.0, half);
        self
    }

    /// The same piece with its sound multiplied by `gain`.
    #[must_use]
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// Seconds the piece plays for.
    pub fn duration(&self) -> f64 {
        (self.end - self.start).max(0.0)
    }
}

/// The pieces of a list and where each starts in the whole.
pub(crate) struct PieceList {
    pieces:    Vec<VideoPiece>,
    /// Seconds of the list every piece starts at, and the whole length as
    /// one more entry.
    starts:    Vec<f64>,
    /// The gain of every piece as the bits of an `f32`. The sound thread
    /// reads them while the main thread writes, see `take_gains`.
    gains:     Vec<AtomicU32>,
    /// At least 1 piece has a sound track. The decode thread finds out.
    has_sound: OnceLock<bool>,
    /// The sound file that plays under all pieces.
    music:     Option<Music>,
}

impl PieceList {
    /// A piece with no length plays nothing and is left out.
    fn new(list: impl Into<VideoPieces>) -> Self {
        let list = list.into();
        let pieces: Vec<VideoPiece> = list
            .pieces
            .into_iter()
            .filter(|piece| piece.duration() > 0.0)
            .map(|piece| VideoPiece {
                start: piece.start.max(0.0),
                ..piece
            })
            .collect();
        let mut starts = vec![0.0];
        for piece in &pieces {
            starts.push(starts[starts.len() - 1] + piece.duration());
        }
        let gains = pieces
            .iter()
            .map(|piece| AtomicU32::new(piece.gain.max(0.0).to_bits()))
            .collect();
        Self {
            pieces,
            starts,
            gains,
            has_sound: OnceLock::new(),
            music: list.music.map(Music::new),
        }
    }

    /// The music under the pieces, none for a list with no music.
    pub(crate) fn music(&self) -> Option<&Music> {
        self.music.as_ref()
    }

    /// What the sound of a piece is multiplied by now.
    pub(crate) fn gain(&self, index: usize) -> f32 {
        self.gains
            .get(index)
            .map_or(1.0, |gain| f32::from_bits(gain.load(Ordering::Relaxed)))
    }

    /// Takes the gains and the music volume of `other` when it has the same
    /// pieces and the same music with only other volumes, and says whether
    /// it did. The sound that is read after this call has the new volumes,
    /// so a volume that changes while a list plays needs no new player.
    pub(crate) fn take_gains(&self, other: &Self) -> bool {
        let same_cut = |(mine, theirs): (&VideoPiece, &VideoPiece)| {
            mine.source.same_as(&theirs.source)
                && mine.start.to_bits() == theirs.start.to_bits()
                && mine.end.to_bits() == theirs.end.to_bits()
                && mine.sound_fade_in.to_bits() == theirs.sound_fade_in.to_bits()
                && mine.sound_fade_out.to_bits() == theirs.sound_fade_out.to_bits()
                && mine.crossfade.to_bits() == theirs.crossfade.to_bits()
        };
        let same_music = match (&self.music, &other.music) {
            (None, None) => true,
            (Some(mine), Some(theirs)) => mine.same_but_volume(theirs),
            _ => false,
        };
        if !same_music
            || self.pieces.len() != other.pieces.len()
            || !self.pieces.iter().zip(&other.pieces).all(same_cut)
        {
            return false;
        }
        for (mine, theirs) in self.gains.iter().zip(&other.gains) {
            mine.store(theirs.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        if let (Some(mine), Some(theirs)) = (&self.music, &other.music) {
            mine.take_volume(theirs);
        }
        true
    }

    pub(crate) fn pieces(&self) -> &[VideoPiece] {
        &self.pieces
    }

    /// Seconds the whole list plays for.
    pub(crate) fn duration(&self) -> f64 {
        self.starts[self.starts.len() - 1]
    }

    /// Seconds of the list the piece starts at, the whole length for an
    /// index past the last piece.
    pub(crate) fn start_of(&self, index: usize) -> f64 {
        self.starts[index.min(self.pieces.len())]
    }

    /// The piece that plays at these seconds of the list and the seconds of
    /// its source that play then. The end of the list is the end of the last
    /// piece. None for a list with no pieces.
    pub(crate) fn locate(&self, seconds: f64) -> Option<(usize, f64)> {
        let last = self.pieces.len().checked_sub(1)?;
        let seconds = seconds.clamp(0.0, self.duration());
        let index = self.starts[1..].iter().position(|end| seconds < *end).unwrap_or(last);
        Some((index, self.pieces[index].start + seconds - self.starts[index]))
    }

    /// None until the decode thread has looked at the files. A list with
    /// music has a sound whatever its pieces have, known at once.
    pub(crate) fn has_sound(&self) -> Option<bool> {
        if self.music.is_some() {
            return Some(true);
        }
        self.has_sound.get().copied()
    }

    pub(crate) fn set_has_sound(&self, has_sound: bool) {
        // The first answer stays, the files are the same the next time.
        self.has_sound.get_or_init(|| has_sound);
    }
}

impl VideoSource {
    /// A source that plays these pieces one after another as 1 video, with
    /// the music of the list under them.
    pub(crate) fn from_pieces(pieces: impl Into<VideoPieces>) -> Self {
        let list = PieceList::new(pieces);
        let music = list.music.as_ref().map_or_else(String::new, |music| {
            format!(", music {}", music.source().location())
        });
        Self {
            location: format!("{} pieces, {:.2} s{music}", list.pieces.len(), list.duration()),
            pieces: Some(Arc::new(list)),
            ..Self::default()
        }
    }

    /// The list of a source made of pieces, none for a file or a url.
    pub(crate) fn piece_list(&self) -> Option<&Arc<PieceList>> {
        self.pieces.as_ref()
    }

    /// Both read the same file the same way, so an open demuxer of one
    /// serves the other.
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.pieces.is_none()
            && other.pieces.is_none()
            && self.location == other.location
            && self.headers == other.headers
    }
}

#[cfg(test)]
mod test {
    use crate::video::{VideoPiece, source::pieces::PieceList};

    fn list() -> PieceList {
        PieceList::new([
            VideoPiece::new("a.mp4", 1.0, 3.0),
            VideoPiece::new("b.mp4", 5.0, 5.0),
            VideoPiece::new("a.mp4", 0.5, 1.0),
            VideoPiece::new("c.mp4", 9.0, 2.0),
        ])
    }

    #[test]
    fn a_piece_with_no_length_is_left_out() {
        let list = list();
        let kept: Vec<_> = list.pieces().iter().map(|piece| piece.source.location()).collect();
        assert_eq!(kept, ["a.mp4", "a.mp4"]);
        assert!((list.duration() - 2.5).abs() < 1e-9);
        assert!((list.start_of(1) - 2.0).abs() < 1e-9);
        assert!((list.start_of(7) - 2.5).abs() < 1e-9);
    }

    /// A list with the same cuts hands over its gains, a list with another
    /// cut does not, that one needs a new player.
    #[test]
    fn only_the_same_pieces_hand_over_their_gains() {
        let piece = |start: f64, gain: f32| VideoPiece::new("a.mp4", start, 3.0).with_gain(gain);
        let list = PieceList::new([piece(1.0, 1.0), piece(2.0, 0.5)]);
        assert!((list.gain(1) - 0.5).abs() < f32::EPSILON);

        assert!(list.take_gains(&PieceList::new([piece(1.0, 4.0), piece(2.0, 0.5)])));
        assert!((list.gain(0) - 4.0).abs() < f32::EPSILON);
        assert!((list.gain(1) - 0.5).abs() < f32::EPSILON);

        assert!(!list.take_gains(&PieceList::new([piece(1.5, 8.0), piece(2.0, 8.0)])));
        assert!(!list.take_gains(&PieceList::new([piece(1.0, 8.0)])));
        let other_file = VideoPiece::new("b.mp4", 2.0, 3.0).with_gain(8.0);
        assert!(!list.take_gains(&PieceList::new([piece(1.0, 8.0), other_file])));
        assert!(
            (list.gain(0) - 4.0).abs() < f32::EPSILON,
            "a list that is refused changes nothing"
        );
    }

    #[test]
    fn a_place_in_the_list_is_a_place_in_a_piece() {
        let list = list();
        let near = |found: Option<(usize, f64)>, index: usize, seconds: f64| {
            let (piece, at) = found.expect("the list has pieces");
            assert_eq!(piece, index);
            assert!((at - seconds).abs() < 1e-9, "{at} is not {seconds}");
        };
        near(list.locate(0.0), 0, 1.0);
        near(list.locate(1.25), 0, 2.25);
        // The place of a cut is the first frame of the piece after it.
        near(list.locate(2.0), 1, 0.5);
        near(list.locate(2.25), 1, 0.75);
        near(list.locate(2.5), 1, 1.0);
        near(list.locate(99.0), 1, 1.0);
        near(list.locate(-1.0), 0, 1.0);
        assert!(PieceList::new([]).locate(0.0).is_none());
    }
}
