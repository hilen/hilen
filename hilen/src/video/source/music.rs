//! The music of a list of pieces: 1 sound file that plays under all pieces,
//! from the start of the list to its end. `VideoPieces` is the list with its
//! music, the value the preview and the export both take.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::video::{VideoPiece, VideoSource};

/// 1 sound file that plays under all pieces of a list, mixed with their
/// sound. It starts with the list and ends with it. A file that ends before
/// the list leaves the rest to the pieces alone, it does not loop. The file
/// needs no picture: ogg, wav, mp3 and flac open, and so does the sound of a
/// video file.
#[derive(Clone)]
pub struct VideoMusic {
    pub source:   VideoSource,
    /// What the music is multiplied by. 1 is the sound of the file, 0 is
    /// silence. A sample that goes past full scale after the mix with the
    /// pieces is cut flat.
    pub volume:   f32,
    /// Seconds of the file that play at the start of the list.
    pub start:    f64,
    /// Seconds from the start of the list over which the music rises from
    /// silence to its volume. A fade follows the square of its time, so
    /// half way the music is at a quarter, which the ear hears as half.
    pub fade_in:  f64,
    /// Seconds before the end of the list over which the music falls to
    /// silence.
    pub fade_out: f64,
}

impl VideoMusic {
    /// The file from its start at its own volume, with no fades.
    pub fn new(source: impl Into<VideoSource>) -> Self {
        Self {
            source:   source.into(),
            volume:   1.0,
            start:    0.0,
            fade_in:  0.0,
            fade_out: 0.0,
        }
    }

    /// The same music multiplied by `volume`.
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// The same music from these seconds of its file.
    #[must_use]
    pub fn with_start(mut self, seconds: f64) -> Self {
        self.start = seconds;
        self
    }

    /// The same music with a fade in over these seconds from the start of
    /// the list.
    #[must_use]
    pub fn with_fade_in(mut self, seconds: f64) -> Self {
        self.fade_in = seconds;
        self
    }

    /// The same music with a fade out over these seconds up to the end of
    /// the list.
    #[must_use]
    pub fn with_fade_out(mut self, seconds: f64) -> Self {
        self.fade_out = seconds;
        self
    }
}

/// A list of pieces with the music under it. A plain list of pieces converts
/// into one with no music, so `set_pieces` and `VideoExport::start` take an
/// array or a `Vec` of pieces too.
#[derive(Clone, Default)]
pub struct VideoPieces {
    pub pieces: Vec<VideoPiece>,
    pub music:  Option<VideoMusic>,
}

impl VideoPieces {
    pub fn new(pieces: impl IntoIterator<Item = VideoPiece>) -> Self {
        Self {
            pieces: pieces.into_iter().collect(),
            music:  None,
        }
    }

    /// The same list with this music under it, or with none.
    #[must_use]
    pub fn with_music(mut self, music: impl Into<Option<VideoMusic>>) -> Self {
        self.music = music.into();
        self
    }
}

impl<I: IntoIterator<Item = VideoPiece>> From<I> for VideoPieces {
    fn from(pieces: I) -> Self {
        Self::new(pieces)
    }
}

/// The music of a list that plays. The volume is the bits of an `f32`, the
/// sound thread reads it while the main thread writes, like the gain of a
/// piece.
pub(crate) struct Music {
    source:   VideoSource,
    volume:   AtomicU32,
    start:    f64,
    fade_in:  f64,
    fade_out: f64,
}

impl Music {
    pub(super) fn new(music: VideoMusic) -> Self {
        Self {
            source:   music.source,
            volume:   AtomicU32::new(music.volume.max(0.0).to_bits()),
            start:    music.start.max(0.0),
            fade_in:  music.fade_in.max(0.0),
            fade_out: music.fade_out.max(0.0),
        }
    }

    pub(crate) fn source(&self) -> &VideoSource {
        &self.source
    }

    /// What the music is multiplied by now.
    pub(crate) fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    /// Seconds of the file that play at the start of the list.
    pub(crate) fn start(&self) -> f64 {
        self.start
    }

    pub(crate) fn fade_in(&self) -> f64 {
        self.fade_in
    }

    pub(crate) fn fade_out(&self) -> f64 {
        self.fade_out
    }

    /// Both are the same file from the same place with the same fades, so
    /// only the volume can differ.
    pub(super) fn same_but_volume(&self, other: &Self) -> bool {
        self.source.same_as(&other.source)
            && self.start.to_bits() == other.start.to_bits()
            && self.fade_in.to_bits() == other.fade_in.to_bits()
            && self.fade_out.to_bits() == other.fade_out.to_bits()
    }

    pub(super) fn take_volume(&self, other: &Self) {
        self.volume.store(other.volume.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}
