//! The sound of a list of pieces as 1 kira streaming decoder, so the step
//! from a piece to the next has no gap: the last sample of a piece and the
//! first of the next one leave in the same stream. Every file is resampled
//! to one rate, a piece with no sound track is silence of its length. The
//! export reads the same stream.

use std::{ptr::null, sync::Arc};

use ffmpeg_next::{
    ChannelLayout, Error, Packet, codec, decoder,
    ffi::swr_convert_frame,
    filter,
    format::{Sample, context::Input, sample::Type},
    frame, media,
    software::resampling,
    util::error::EAGAIN,
};
use kira::{Frame, sound::streaming::Decoder};

use crate::{
    gm::LossyConvert,
    video::{
        VideoSource,
        audio::{SPEEDS, TAIL, drain, push_samples, tempo_graph},
        count_to_f64,
        source::{Interrupt, PieceList},
    },
};

/// The sample rate every piece is brought to.
pub(crate) const RATE: u32 = 48_000;

/// Frames of silence in one chunk of a piece with no sound.
const SILENCE: usize = 4096;

/// How far after the place asked for a stream may start and still have the
/// time before it filled with silence, in samples. Further off is not a gap
/// to fill.
const GAP: usize = 48_000;

/// Seconds before the place asked for that a seek of a file aims at. A
/// demuxer seeks by the picture, and matroska then starts at the cluster of
/// a keyframe, where the first sound packet can lie some hundredths of a
/// second after the keyframe. Aimed at the place itself, the sound after a
/// cut started with that much silence, 44 ms in the `tracks.mkv` fixture.
const SEEK_BEFORE: f64 = 0.25;

fn samples(seconds: f64) -> usize {
    (seconds.max(0.0) * f64::from(RATE)).round().lossy_convert()
}

fn seconds(samples: usize) -> f64 {
    count_to_f64(u64::try_from(samples).expect("a sample count fits u64")) / f64::from(RATE)
}

/// The sound track of one file, decoded to stereo floats at `RATE`.
struct FileSound {
    source:    VideoSource,
    input:     Input,
    stream:    usize,
    time_base: f64,
    decoder:   decoder::Audio,
    resampler: resampling::Context,
    reads:     Interrupt,
    /// Decoded and not handed out yet.
    pending:   Vec<Frame>,
    eof:       bool,
    /// The sample of the file the next `read` starts at.
    at:        usize,
}

impl FileSound {
    /// None when the file has no sound track.
    fn open(source: &VideoSource, reads: &Interrupt) -> Result<Option<Self>, Error> {
        let input = source.open(reads)?;
        let Some(stream) = input.streams().best(media::Type::Audio) else {
            return Ok(None);
        };
        let index = stream.index();
        let time_base: f64 = stream.time_base().into();
        let context = codec::context::Context::from_parameters(stream.parameters())?;
        let decoder = context.decoder().audio()?;
        let resampler = resampler(&decoder)?;
        Ok(Some(Self {
            source: source.clone(),
            input,
            stream: index,
            time_base,
            decoder,
            resampler,
            reads: reads.clone(),
            pending: Vec::new(),
            eof: false,
            at: 0,
        }))
    }

    /// The next `read` starts at this sample of the file. The demuxer lands
    /// on the packet before it and the sound up to the sample is dropped.
    fn seek(&mut self, sample: usize) -> Result<(), Error> {
        let before = (seconds(sample) - SEEK_BEFORE).max(0.0);
        let micros: i64 = (before * 1_000_000.0).round().lossy_convert();
        {
            let _reading = self.reads.reading();
            self.input.seek(micros, ..micros)?;
        }
        self.decoder.flush();
        // The old one holds sound from before the seek.
        self.resampler = resampler(&self.decoder)?;
        self.pending.clear();
        self.eof = false;
        self.at = sample;

        let mut landed = None;
        while landed.is_none() && !self.eof {
            landed = self.decode_more()?;
        }
        let Some(landed) = landed else {
            return Ok(());
        };
        let landed = samples(landed);
        if landed >= sample {
            // A stream whose sound starts a little after the place.
            let gap = landed - sample;
            if gap <= GAP {
                self.pending.splice(0..0, vec![Frame::ZERO; gap]);
            }
            return Ok(());
        }
        let mut skip = sample - landed;
        while skip > 0 {
            if self.pending.is_empty() {
                if self.eof {
                    break;
                }
                self.decode_more()?;
                continue;
            }
            let dropped = skip.min(self.pending.len());
            self.pending.drain(..dropped);
            skip -= dropped;
        }
        Ok(())
    }

    /// At most `most` frames from where the file stands. Past the end of
    /// its sound the file gives silence, a piece can be longer than the
    /// sound of its file.
    fn read(&mut self, most: usize) -> Result<Vec<Frame>, Error> {
        while self.pending.is_empty() && !self.eof {
            self.decode_more()?;
        }
        if self.pending.is_empty() {
            let count = most.min(SILENCE);
            self.at += count;
            return Ok(vec![Frame::ZERO; count]);
        }
        let count = most.min(self.pending.len());
        self.at += count;
        Ok(self.pending.drain(..count).collect())
    }

    /// One more packet through the decoder into `pending`. Returns the
    /// timestamp in seconds of the first sound that came out of it.
    fn decode_more(&mut self) -> Result<Option<f64>, Error> {
        if let Some(packet) = self.next_packet()? {
            self.decoder.send_packet(&packet)?;
        } else {
            self.eof = true;
            self.decoder.send_eof()?;
        }
        let mut first = None;
        loop {
            let mut decoded = frame::Audio::empty();
            match self.decoder.receive_frame(&mut decoded) {
                Ok(()) => {}
                Err(Error::Other { errno: EAGAIN } | Error::Eof) => break,
                Err(err) => return Err(err),
            }
            if first.is_none()
                && let Some(pts) = decoded.pts().or(decoded.timestamp())
            {
                let ticks: f64 = pts.lossy_convert();
                first = Some(ticks * self.time_base);
            }
            resample(&mut self.resampler, Some(&decoded), &mut self.pending)?;
        }
        if self.eof {
            // What the resampler still holds.
            resample(&mut self.resampler, None, &mut self.pending)?;
        }
        Ok(first)
    }

    /// The next packet of the sound stream, None at the end.
    fn next_packet(&mut self) -> Result<Option<Packet>, Error> {
        loop {
            let mut packet = Packet::empty();
            let read = {
                let _reading = self.reads.reading();
                packet.read(&mut self.input)
            };
            match read {
                Ok(()) if packet.stream() == self.stream => return Ok(Some(packet)),
                Ok(()) => {}
                Err(Error::Eof) => return Ok(None),
                Err(err) => return Err(err),
            }
        }
    }
}

fn resampler(decoder: &decoder::Audio) -> Result<resampling::Context, Error> {
    let layout = if decoder.channel_layout().is_empty() {
        ChannelLayout::default(i32::from(decoder.channels()))
    } else {
        decoder.channel_layout()
    };
    resampling::Context::get(
        decoder.format(),
        layout,
        decoder.rate(),
        Sample::F32(Type::Packed),
        ChannelLayout::STEREO,
        RATE,
    )
}

/// Resamples a decoded frame into `out`, or with no frame what the resampler
/// still holds. The output frame is left for swresample to size: when the
/// rate goes up more samples come out than went in, and a frame sized like
/// the input would leave the rest inside the resampler for good.
fn resample(
    resampler: &mut resampling::Context,
    decoded: Option<&frame::Audio>,
    out: &mut Vec<Frame>,
) -> Result<(), Error> {
    let mut converted = frame::Audio::empty();
    converted.set_format(Sample::F32(Type::Packed));
    converted.set_channel_layout(ChannelLayout::STEREO);
    converted.set_rate(RATE);
    // SAFETY: the context and both frames are valid, a null input asks for
    // the samples the context holds, and swresample allocates the output.
    let code = unsafe {
        swr_convert_frame(
            resampler.as_mut_ptr(),
            converted.as_mut_ptr(),
            decoded.map_or(null(), |frame| frame.as_ptr()),
        )
    };
    if code < 0 {
        return Err(Error::from(code));
    }
    if converted.samples() > 0 {
        push_samples(out, &converted);
    }
    Ok(())
}

/// Stereo floats as a frame the tempo filter takes.
fn sound_frame(frames: &[Frame]) -> frame::Audio {
    let mut sound = frame::Audio::new(Sample::F32(Type::Packed), frames.len(), ChannelLayout::STEREO);
    sound.set_rate(RATE);
    let bytes = &mut sound.data_mut(0)[..frames.len() * 2 * size_of::<f32>()];
    let floats: &mut [f32] = bytemuck::cast_slice_mut(bytes);
    for (pair, frame) in floats.as_chunks_mut::<2>().0.iter_mut().zip(frames) {
        pair[0] = frame.left;
        pair[1] = frame.right;
    }
    sound
}

/// The file that is open, for the piece that is read.
struct Open {
    piece: usize,
    /// None for a file with no sound track.
    file:  Option<FileSound>,
}

pub(crate) struct PiecesSound {
    list:   Arc<PieceList>,
    reads:  Interrupt,
    /// The sample of the list every piece starts at, and the whole length
    /// as one more entry.
    starts: Vec<usize>,
    /// The sample of the list the next read gives.
    at:     usize,
    open:   Option<Open>,
    /// How fast the sound plays, see `AudioDecoder`.
    speed:  f64,
    /// Changes the speed and keeps the pitch, none at speed 1.
    tempo:  Option<filter::Graph>,
    /// The tempo filter got the end of the list.
    closed: bool,
}

impl PiecesSound {
    /// Opens no file, the first read does, on the thread that reads.
    pub(crate) fn new(list: &Arc<PieceList>, speed: f64, reads: Interrupt) -> Result<Self, Error> {
        let starts = (0..=list.pieces().len()).map(|index| samples(list.start_of(index))).collect();
        let speed = speed.clamp(SPEEDS.0, SPEEDS.1);
        let tempo = if (speed - 1.0).abs() < f64::EPSILON {
            None
        } else {
            Some(tempo_graph(RATE, speed)?)
        };
        Ok(Self {
            list: Arc::clone(list),
            reads,
            starts,
            at: 0,
            open: None,
            speed,
            tempo,
            closed: false,
        })
    }

    /// Samples in the whole list at its own speed.
    pub(crate) fn total(&self) -> usize {
        self.starts[self.starts.len() - 1]
    }

    /// The next read gives this sample of the list.
    pub(crate) fn seek_list(&mut self, sample: usize) {
        // The next read brings the file to the place.
        self.at = sample.min(self.total());
    }

    /// The next frames of the list at its own speed, none at its end.
    pub(crate) fn read_list(&mut self) -> Result<Vec<Frame>, Error> {
        let Some(piece) = self.starts[1..].iter().position(|end| self.at < *end) else {
            return Ok(Vec::new());
        };
        let left = self.starts[piece + 1] - self.at;
        let place = samples(self.list.pieces()[piece].start) + self.at - self.starts[piece];
        self.open_at(piece, place)?;

        let frames = match self.open.as_mut().and_then(|open| open.file.as_mut()) {
            Some(file) => file.read(left)?,
            // The file has no sound track.
            None => vec![Frame::ZERO; left.min(SILENCE)],
        };
        self.at += frames.len();
        Ok(frames)
    }

    /// Makes the open file the one of this piece, standing at `place`. A
    /// piece that goes on where the one before it ended in the same file
    /// reads on with no seek, so not a sample is lost or doubled at the cut.
    fn open_at(&mut self, piece: usize, place: usize) -> Result<(), Error> {
        let source = &self.list.pieces()[piece].source;
        let mut file = match self.open.take() {
            Some(open) if open.piece == piece => open.file,
            Some(Open { file: Some(file), .. }) if file.source.same_as(source) => Some(file),
            _ => FileSound::open(source, &self.reads)?,
        };
        if let Some(file) = &mut file
            && file.at != place
        {
            file.seek(place)?;
        }
        self.open = Some(Open { piece, file });
        Ok(())
    }

    /// The next frames at the speed of the sound, through the tempo filter.
    fn read_at_speed(&mut self) -> Result<Vec<Frame>, Error> {
        let mut out = Vec::new();
        while out.is_empty() && !self.closed {
            let frames = self.read_list()?;
            let Some(tempo) = &mut self.tempo else {
                return Ok(frames);
            };
            if frames.is_empty() {
                self.closed = true;
                tempo.get("in").ok_or(Error::FilterNotFound)?.source().flush()?;
            } else {
                tempo
                    .get("in")
                    .ok_or(Error::FilterNotFound)?
                    .source()
                    .add(&sound_frame(&frames))?;
            }
            drain(tempo, &mut out)?;
        }
        Ok(out)
    }
}

impl Decoder for PiecesSound {
    type Error = Error;

    fn sample_rate(&self) -> u32 {
        RATE
    }

    fn num_frames(&self) -> usize {
        (seconds(self.total()) * f64::from(RATE) / self.speed).ceil().lossy_convert()
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Error> {
        let mut frames = self.read_at_speed()?;
        if frames.is_empty() {
            // kira walks chunk by chunk, an empty one would stop it.
            frames.resize(TAIL, Frame::ZERO);
        }
        Ok(frames)
    }

    /// Reads nothing, the next `decode` starts at the sample, exactly. kira
    /// runs the first seek on the main thread.
    fn seek(&mut self, index: usize) -> Result<usize, Error> {
        // kira counts samples of the sound it hears, the list is `speed`
        // times that far along.
        let place = seconds(index) * f64::from(RATE) * self.speed;
        self.seek_list(place.round().lossy_convert());
        self.closed = false;
        if self.tempo.is_some() {
            // The filter holds sound from before the seek.
            self.tempo = Some(tempo_graph(RATE, self.speed)?);
        }
        Ok(index)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use kira::{Frame, sound::streaming::Decoder};

    use crate::{
        gm::LossyConvert,
        video::{
            VideoPiece, VideoSource,
            audio::pieces::{PiecesSound, seconds},
            source::Interrupt,
            test_fixture,
        },
    };

    fn sound(pieces: impl IntoIterator<Item = VideoPiece>, speed: f64) -> PiecesSound {
        let source = VideoSource::from_pieces(pieces);
        let stop = Arc::new(AtomicBool::new(false));
        let list = source.piece_list().expect("a list of pieces");
        PiecesSound::new(list, speed, Interrupt::new(&stop)).expect("the sound is made")
    }

    fn ramp(start: f64, end: f64) -> VideoPiece {
        VideoPiece::new(test_fixture("ramp.mp4"), start, end)
    }

    /// Everything the sound gives up to its end.
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

    /// The sound of the ramp fixture rises in a line from -0.8 to 0.8 over
    /// its 2 seconds, so a sample tells the place of the file it is from.
    /// One sample further the value is 0.000017 higher.
    fn ramp_at(sample: usize) -> f32 {
        (-0.8 + 0.8 * seconds(sample)).lossy_convert()
    }

    /// Checks that `frames` are the samples of the ramp file from `first`
    /// on, exact to the sample.
    fn assert_ramp(frames: &[Frame], first: usize, what: &str) {
        for (index, frame) in frames.iter().enumerate() {
            let expected = ramp_at(first + index);
            assert!(
                (frame.left - expected).abs() < 0.000_006 && (frame.right - expected).abs() < 0.000_006,
                "{what}: sample {index} is {}, the file has {expected} at sample {}",
                frame.left,
                first + index
            );
        }
    }

    /// 2 pieces of 1 file with a jump between them. Every sample of the
    /// first piece is there up to its last one, and the very next sample is
    /// the first one of the second piece: no gap, nothing lost, nothing
    /// played twice.
    #[test]
    fn a_cut_inside_one_file_has_no_gap() {
        let mut sound = sound([ramp(0.25, 0.5), ramp(1.0, 1.25)], 1.0);
        assert_eq!(sound.num_frames(), 24_000);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 24_000);
        assert_ramp(&frames[..12_000], 12_000, "the first piece");
        assert_ramp(&frames[12_000..], 48_000, "the second piece");
    }

    /// A piece that goes on where the one before it ended, the cut of a
    /// split with no trim. The sound is the sound of the file with no cut.
    #[test]
    fn a_split_with_no_trim_reads_on() {
        let mut sound = sound([ramp(0.25, 0.5), ramp(0.5, 0.75)], 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 24_000);
        assert_ramp(&frames, 12_000, "both pieces");
    }

    /// A piece of a file with no sound track is silence of its exact
    /// length, and the piece after it starts on its own first sample.
    #[test]
    fn a_piece_with_no_sound_is_silence() {
        let silent = VideoPiece::new(test_fixture("colors.mp4"), 1.0, 1.5);
        let mut sound = sound([ramp(0.0, 0.1), silent, ramp(1.5, 1.6)], 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 4_800 + 24_000 + 4_800);
        assert_ramp(&frames[..4_800], 0, "the first piece");
        assert!(
            frames[4_800..28_800].iter().all(|frame| *frame == Frame::ZERO),
            "the piece with no sound is not silent"
        );
        assert_ramp(&frames[28_800..], 72_000, "the last piece");
    }

    /// A seek reads nothing and the next decode starts on the sample asked
    /// for, in whatever piece it is. A seek back works the same.
    #[test]
    fn a_seek_goes_to_a_sample_of_the_whole_list() {
        let mut sound = sound([ramp(0.25, 0.5), ramp(1.0, 1.25)], 1.0);
        for (sample, in_file) in [(15_000, 51_000), (3, 12_003), (11_999, 23_999), (12_000, 48_000)] {
            assert_eq!(sound.seek(sample).expect("the seek"), sample);
            let frames = sound.decode().expect("the fixture decodes");
            assert_ramp(&frames[..1], in_file, "the first sample after the seek");
        }
        // The last sample of the first piece is followed by the cut.
        sound.seek(11_999).expect("the seek");
        let mut frames = Vec::new();
        while frames.len() < 2 {
            frames.extend(sound.decode().expect("the fixture decodes"));
        }
        assert_ramp(&frames[..1], 23_999, "before the cut");
        assert_ramp(&frames[1..2], 48_000, "after the cut");
    }

    /// A file at another sample rate is brought to the rate of the list. The
    /// fixture is a 440 Hz tone at 44100 Hz, a piece of 1 second of it must
    /// be 48000 samples of 440 Hz. Played at the wrong rate it is 479 Hz.
    /// It is a matroska file too: with the seek aimed at the place itself
    /// the piece started with 44 ms of silence, 841 sign changes.
    #[test]
    fn a_file_at_another_rate_is_resampled() {
        let tone = VideoPiece::new(test_fixture("tracks.mkv"), 1.0, 2.0);
        let mut sound = sound([tone], 1.0);
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 48_000);
        let crossings = frames
            .windows(2)
            .filter(|pair| (pair[0].left < 0.0) != (pair[1].left < 0.0))
            .count();
        assert!(
            crossings.abs_diff(880) <= 6,
            "the tone has {crossings} sign changes in a second, 440 Hz has 880"
        );
    }

    /// At double speed the list is half as long for kira and a seek to a
    /// sample kira hears is twice as far into the list.
    #[test]
    fn speed_changes_the_length() {
        let mut sound = sound([ramp(0.0, 1.0), ramp(1.0, 2.0)], 2.0);
        assert_eq!(sound.num_frames(), 48_000);
        let frames = all(&mut sound);
        assert!(
            frames.len().abs_diff(48_000) < 2_400,
            "at double speed 2 seconds are {} samples",
            frames.len()
        );
        sound.seek(24_000).expect("the seek");
        assert_eq!(sound.at, 48_000);
    }
}
