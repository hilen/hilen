//! A list of pieces that plays as 1 video. Each piece is a source with the
//! seconds of it that play. The list travels inside a `VideoSource`, so the
//! player, its decode thread and its sound get it the way they get a file.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU32, Ordering},
};

use crate::video::VideoSource;

/// One piece of a list that plays as 1 video: a source and the part of it
/// that plays, from `start` up to `end`, in seconds of that source. The
/// frame at `end` is not part of the piece, it is the first frame of a piece
/// that starts there.
#[derive(Clone)]
pub struct VideoPiece {
    pub source: VideoSource,
    pub start:  f64,
    pub end:    f64,
    /// What the sound of the piece is multiplied by. 1 is the sound of the
    /// source, 2 is twice as loud, 0 is silence. A sample that goes past
    /// full scale is cut flat. The picture is not changed.
    pub gain:   f32,
}

impl VideoPiece {
    pub fn new(source: impl Into<VideoSource>, start: f64, end: f64) -> Self {
        Self {
            source: source.into(),
            start,
            end,
            gain: 1.0,
        }
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
}

impl PieceList {
    /// A piece with no length plays nothing and is left out.
    fn new(pieces: impl IntoIterator<Item = VideoPiece>) -> Self {
        let pieces: Vec<VideoPiece> = pieces
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
        }
    }

    /// What the sound of a piece is multiplied by now.
    pub(crate) fn gain(&self, index: usize) -> f32 {
        self.gains
            .get(index)
            .map_or(1.0, |gain| f32::from_bits(gain.load(Ordering::Relaxed)))
    }

    /// Takes the gains of `other` when it has the same pieces with only
    /// other gains, and says whether it did. The sound that is read after
    /// this call has the new gains, so a volume that changes while a list
    /// plays needs no new player.
    pub(crate) fn take_gains(&self, other: &Self) -> bool {
        let same_cut = |(mine, theirs): (&VideoPiece, &VideoPiece)| {
            mine.source.same_as(&theirs.source)
                && mine.start.to_bits() == theirs.start.to_bits()
                && mine.end.to_bits() == theirs.end.to_bits()
        };
        if self.pieces.len() != other.pieces.len() || !self.pieces.iter().zip(&other.pieces).all(same_cut) {
            return false;
        }
        for (mine, theirs) in self.gains.iter().zip(&other.gains) {
            mine.store(theirs.load(Ordering::Relaxed), Ordering::Relaxed);
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

    /// None until the decode thread has looked at the files.
    pub(crate) fn has_sound(&self) -> Option<bool> {
        self.has_sound.get().copied()
    }

    pub(crate) fn set_has_sound(&self, has_sound: bool) {
        // The first answer stays, the files are the same the next time.
        self.has_sound.get_or_init(|| has_sound);
    }
}

impl VideoSource {
    /// A source that plays these pieces one after another as 1 video.
    pub(crate) fn from_pieces(pieces: impl IntoIterator<Item = VideoPiece>) -> Self {
        let list = PieceList::new(pieces);
        Self {
            location: format!("{} pieces, {:.2} s", list.pieces.len(), list.duration()),
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
