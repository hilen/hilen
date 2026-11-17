//! The music of a list of pieces, mixed into the sound of the pieces where
//! the list is read. So the preview and the export both have it, a seek of
//! the list takes the music along, and the tempo filter after the read
//! keeps it in step at another speed.

use ffmpeg_next::Error;
use kira::Frame;
use log::warn;

use crate::{
    gm::LossyConvert,
    video::{
        audio::pieces::{FileSound, samples, seconds},
        source::{Interrupt, Music},
    },
};

/// The fade in and the fade out of the music, in samples of the list.
struct Fade {
    /// Samples in the whole list.
    total: usize,
    rise:  usize,
    fall:  usize,
}

impl Fade {
    fn new(music: &Music, total: usize) -> Self {
        Self {
            total,
            rise: samples(music.fade_in()),
            fall: samples(music.fade_out()),
        }
    }

    /// What the music is multiplied by at this sample of the list, 0 to 1.
    /// The fade out counts back from the end of the list, not of the file.
    /// The factor is the square of the part of the fade that is done. A
    /// straight line is not heard as a fade until its last part: half way
    /// it is only 6 decibels down. The square is 12 down there, and the ear
    /// hears an even fall.
    fn at(&self, sample: usize) -> f32 {
        let mut factor = 1.0;
        if sample < self.rise {
            let part = seconds(sample) / seconds(self.rise);
            factor *= part * part;
        }
        let left = self.total.saturating_sub(sample);
        if left < self.fall {
            let part = seconds(left) / seconds(self.fall);
            factor *= part * part;
        }
        factor.lossy_convert()
    }
}

/// The open file of the music of a list.
#[derive(Default)]
pub(super) enum MusicSound {
    /// The first read opens the file, on the thread that reads.
    #[default]
    Unopened,
    Open(Box<FileSound>),
    /// The file has no sound track, the pieces play alone.
    NoSound,
}

impl MusicSound {
    /// Adds the music to `frames`, the sound of the pieces from the sample
    /// `at` of a list of `total` samples. What goes past full scale is cut
    /// flat. Past the end of its file the music is silence.
    pub(super) fn mix(
        &mut self,
        music: &Music,
        frames: &mut [Frame],
        at: usize,
        total: usize,
        reads: &Interrupt,
    ) -> Result<(), Error> {
        if matches!(self, Self::Unopened) {
            // A demuxer of its own, so it gets a read mark of its own.
            *self = if let Some(file) = FileSound::open(music.source(), &reads.fresh())? {
                Self::Open(Box::new(file))
            } else {
                warn!(
                    "video music {}: the file has no sound track",
                    music.source().location()
                );
                Self::NoSound
            };
        }
        let Self::Open(file) = self else {
            return Ok(());
        };
        // After a seek of the list, and on the first read of a music that
        // starts inside its file.
        let place = samples(music.start()) + at;
        if file.at != place {
            file.seek(place)?;
        }

        let fade = Fade::new(music, total);
        let volume = music.volume();
        let mut done = 0;
        while done < frames.len() {
            let chunk = file.read(frames.len() - done)?;
            if chunk.is_empty() {
                break;
            }
            for (index, (frame, sound)) in frames[done..].iter_mut().zip(&chunk).enumerate() {
                let factor = volume * fade.at(at + done + index);
                frame.left = (frame.left + sound.left * factor).clamp(-1.0, 1.0);
                frame.right = (frame.right + sound.right * factor).clamp(-1.0, 1.0);
            }
            done += chunk.len();
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        f64::consts::TAU,
        sync::{Arc, atomic::AtomicBool},
    };

    use kira::{Frame, sound::streaming::Decoder};

    use crate::{
        gm::LossyConvert,
        video::{
            VideoMusic, VideoPiece, VideoPieces, VideoSource,
            audio::pieces::{PiecesSound, seconds},
            source::Interrupt,
            test_fixture, test_wav,
        },
    };

    fn sound(pieces: impl IntoIterator<Item = VideoPiece>, music: VideoMusic, speed: f64) -> PiecesSound {
        let source = VideoSource::from_pieces(VideoPieces::new(pieces).with_music(music));
        let stop = Arc::new(AtomicBool::new(false));
        let list = source.piece_list().expect("a list of pieces");
        PiecesSound::new(list, speed, Interrupt::new(&stop)).expect("the sound is made")
    }

    fn ramp(start: f64, end: f64) -> VideoPiece {
        VideoPiece::new(test_fixture("ramp.mp4"), start, end)
    }

    /// A piece of a file with no sound track.
    fn mute(start: f64, end: f64) -> VideoPiece {
        VideoPiece::new(test_fixture("colors.mp4"), start, end)
    }

    /// The sample of the ramp fixture at a place, see `pieces.rs`.
    fn ramp_at(sample: usize) -> f32 {
        (-0.8 + 0.8 * seconds(sample)).lossy_convert()
    }

    /// The sample `index` of the rising music file as a wav file holds it.
    /// It goes up by 1 step of 16 bits every 4 samples, so a sample tells
    /// its place in the file.
    fn rising_step(index: usize) -> i16 {
        i16::try_from(index / 4).expect("the music file is short")
    }

    /// A music file of `length` samples that rises, see `rising_step`.
    fn rising(name: &str, length: usize) -> VideoMusic {
        let steps: Vec<i16> = (0..length).map(rising_step).collect();
        VideoMusic::new(test_wav(name, &steps))
    }

    /// The sample of the rising music file at a place, as the list hears it.
    fn rising_at(sample: usize) -> f32 {
        f32::from(rising_step(sample)) / 32_768.0
    }

    /// A music file of 2 seconds that is 0.5 all along.
    fn flat(name: &str) -> VideoMusic {
        VideoMusic::new(test_wav(name, &vec![16_384; 96_000]))
    }

    /// Everything the list gives up to its end.
    fn all(sound: &mut PiecesSound) -> Vec<Frame> {
        let mut frames = Vec::new();
        loop {
            let chunk = sound.read_at_speed().expect("the fixtures decode");
            if chunk.is_empty() {
                return frames;
            }
            frames.extend(chunk);
        }
    }

    fn assert_near(frame: Frame, expected: f32, what: &str) {
        assert!(
            (frame.left - expected).abs() < 0.000_03 && (frame.right - expected).abs() < 0.000_03,
            "{what}: {} is not {expected}",
            frame.left
        );
    }

    /// Every sample of a list with music is the sample of the piece plus
    /// the sample of the music at its volume.
    #[test]
    fn music_is_added_to_the_pieces() {
        let music = rising("added", 96_000).with_volume(0.5);
        let mut sound = sound([ramp(1.0, 1.25), ramp(0.5, 0.75)], music, 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 24_000);
        for (index, frame) in frames.iter().enumerate() {
            let piece = if index < 12_000 {
                48_000 + index
            } else {
                12_000 + index
            };
            let expected = ramp_at(piece) + 0.5 * rising_at(index);
            assert_near(*frame, expected, &format!("sample {index}"));
        }
    }

    /// Pieces at a gain of 0 and a piece with no sound track give the music
    /// alone, and so does a list where no file has a sound track at all.
    #[test]
    fn silent_pieces_give_the_music_alone() {
        let music = rising("alone", 96_000);
        let mut quiet = sound([ramp(1.0, 1.25).with_gain(0.0), mute(1.0, 1.25)], music, 1.0);
        let frames = all(&mut quiet);
        assert_eq!(frames.len(), 24_000);
        for (index, frame) in frames.iter().enumerate() {
            assert_near(*frame, rising_at(index), &format!("sample {index}"));
        }

        let mut trackless = sound([mute(0.0, 0.5)], rising("alone-mute", 96_000), 1.0);
        assert_eq!(trackless.list.has_sound(), Some(true));
        let frames = all(&mut trackless);
        assert_eq!(frames.len(), 24_000);
        assert_near(frames[20_000], rising_at(20_000), "a list with no sound track");
    }

    /// The fade in counts from the start of the list. The fade out counts
    /// back from the end of the list, the music file is a second longer.
    #[test]
    fn the_fades_follow_the_list() {
        let music = flat("fades").with_fade_in(0.25).with_fade_out(0.5);
        let mut sound = sound([mute(0.0, 1.0)], music, 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 48_000);
        for (sample, expected) in [
            (0, 0.0),
            // Half way through a fade the music is at a quarter.
            (6_000, 0.125),
            (12_000, 0.5),
            (20_000, 0.5),
            (24_000, 0.5),
            (36_000, 0.125),
            (42_000, 0.031_25),
            (47_999, 0.5 / (24_000.0 * 24_000.0)),
        ] {
            assert_near(frames[sample], expected, &format!("sample {sample}"));
        }
        // It rises all the way up and falls all the way down.
        assert!(frames[..12_000].windows(2).all(|pair| pair[0].left < pair[1].left));
        assert!(frames[24_000..].windows(2).all(|pair| pair[0].left > pair[1].left));
    }

    /// The music starts at its own place in the file.
    #[test]
    fn the_music_starts_at_its_place_in_the_file() {
        let music = rising("start", 96_000).with_start(0.5);
        let mut sound = sound([ramp(1.0, 1.25).with_gain(0.0)], music, 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 12_000);
        for (index, frame) in frames.iter().enumerate() {
            assert_near(*frame, rising_at(24_000 + index), &format!("sample {index}"));
        }
    }

    /// A seek of the list lands the music on the same sample of the list,
    /// forward and back.
    #[test]
    fn a_seek_takes_the_music_along() {
        let music = rising("seek", 96_000).with_start(0.25);
        let mut sound = sound([ramp(0.0, 1.0)], music, 1.0);
        for sample in [30_000, 7, 47_999, 12_000] {
            assert_eq!(sound.seek(sample).expect("the seek"), sample);
            let frames = sound.decode().expect("the fixtures decode");
            let expected = ramp_at(sample) + rising_at(12_000 + sample);
            assert_near(
                frames[0],
                expected,
                &format!("the first sample after a seek to {sample}"),
            );
        }
    }

    /// The music file is half a second, the list a whole one. After the end
    /// of the file the pieces go on alone, also after a seek past it, and a
    /// seek back brings the music back.
    #[test]
    fn after_the_music_file_the_pieces_go_on_alone() {
        let mut sound = sound([ramp(1.0, 2.0)], rising("short", 24_000), 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 48_000);
        for (index, frame) in frames.iter().enumerate() {
            let music = if index < 24_000 { rising_at(index) } else { 0.0 };
            assert_near(
                *frame,
                ramp_at(48_000 + index) + music,
                &format!("sample {index}"),
            );
        }

        sound.seek(40_000).expect("the seek");
        let frames = sound.decode().expect("the fixtures decode");
        assert_near(frames[0], ramp_at(88_000), "past the end of the music");
        sound.seek(20_000).expect("the seek");
        let frames = sound.decode().expect("the fixtures decode");
        assert_near(
            frames[0],
            ramp_at(68_000) + rising_at(20_000),
            "back in the music",
        );
    }

    /// The end of the ramp is 0.8 loud and the music adds 0.5. The part of
    /// the sum over full scale is cut flat.
    #[test]
    fn a_mix_past_full_scale_is_cut_flat() {
        let mut sound = sound([ramp(1.5, 2.0)], flat("flat"), 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 24_000);
        assert_near(frames[0], 0.9, "the first sample");
        // The piece reaches 0.5, full scale with the music, after an
        // eighth of a second, 6000 samples.
        assert!(frames[5_900].left < 1.0);
        let full = 1.0_f32.to_bits();
        assert!(
            frames[6_100..]
                .iter()
                .all(|frame| frame.left.to_bits() == full && frame.right.to_bits() == full)
        );
    }

    /// A new volume of the music is in the very next read of the list that
    /// plays. A list with another music is refused, that one needs a new
    /// player.
    #[test]
    fn a_new_music_volume_is_heard_in_the_next_read() {
        let music = flat("volume");
        let mut sound = sound([mute(0.0, 1.0)], music.clone(), 1.0);
        let before = sound.read_list().expect("the fixtures decode");
        assert_near(before[0], 0.5, "before the change");

        let other = |music: Option<VideoMusic>| {
            VideoSource::from_pieces(VideoPieces::new([mute(0.0, 1.0)]).with_music(music))
        };
        let quiet = other(Some(music.clone().with_volume(0.25)));
        assert!(sound.list.take_gains(quiet.piece_list().expect("a list of pieces")));
        let after = sound.read_list().expect("the fixtures decode");
        assert_near(after[0], 0.125, "after the change");

        for refused in [
            other(None),
            other(Some(music.clone().with_start(0.5))),
            other(Some(music.clone().with_fade_in(0.5))),
            other(Some(music.clone().with_fade_out(0.5))),
            other(Some(flat("volume-other"))),
        ] {
            assert!(!sound.list.take_gains(refused.piece_list().expect("a list of pieces")));
        }
        let no_music = VideoSource::from_pieces([mute(0.0, 1.0)]);
        let list = no_music.piece_list().expect("a list of pieces");
        assert!(!list.take_gains(&sound.list), "a list with no music takes none");
        let next = sound.read_list().expect("the fixtures decode");
        assert_near(next[0], 0.125, "a list that is refused changes nothing");
    }

    /// At double speed the music is half as long, like the pieces, since it
    /// goes through the same tempo filter.
    #[test]
    fn the_music_keeps_step_at_another_speed() {
        let mut sound = sound([mute(0.0, 1.0)], flat("speed"), 2.0);
        assert_eq!(sound.num_frames(), 24_000);
        let frames = all(&mut sound);
        assert!(
            frames.len().abs_diff(24_000) < 2_400,
            "at double speed 1 second is {} samples",
            frames.len()
        );
        assert!(
            (frames[12_000].left - 0.5).abs() < 0.01,
            "{}",
            frames[12_000].left
        );
    }

    /// A sound file with no picture opens as music. Each fixture is 1
    /// second of a 440 Hz tone at half of full scale, written by the ffmpeg
    /// command line tool. The 2 lossy ones come back a little off.
    #[test]
    fn a_sound_file_with_no_picture_is_music() {
        let wav: Vec<i16> = (0..48_000_usize)
            .map(|index| {
                let wave = (seconds(index) * 440.0 * TAU).sin();
                let step: i32 = (wave * 16_384.0).round().lossy_convert();
                i16::try_from(step).expect("half of full scale fits 16 bits")
            })
            .collect();
        let files = [
            ("wav", test_wav("tone", &wav)),
            ("flac", test_fixture("tone.flac")),
            ("mp3", test_fixture("tone.mp3")),
            ("ogg with vorbis", test_fixture("tone.ogg")),
            ("ogg with opus", test_fixture("tone_opus.ogg")),
        ];
        for (what, file) in files {
            let mut sound = sound([mute(0.0, 1.0)], VideoMusic::new(file), 1.0);
            let frames = all(&mut sound);
            assert_eq!(frames.len(), 48_000, "{what}");
            // The middle, an encoder may start and end with a little
            // silence.
            let middle = &frames[12_000..36_000];
            let peak = middle.iter().map(|frame| frame.left.abs()).fold(0.0, f32::max);
            assert!((peak - 0.5).abs() < 0.05, "{what}: the peak is {peak}");
            let crossings = middle
                .windows(2)
                .filter(|pair| (pair[0].left < 0.0) != (pair[1].left < 0.0))
                .count();
            assert!(
                crossings.abs_diff(440) <= 4,
                "{what}: {crossings} sign changes in half a second, 440 Hz has 440"
            );
        }
    }
}
