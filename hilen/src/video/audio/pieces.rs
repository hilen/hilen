//! The sound of a list of pieces as 1 kira streaming decoder, so the step
//! from a piece to the next has no gap: the last sample of a piece and the
//! first of the next one leave in the same stream. Every file is resampled
//! to one rate, a piece with no sound track is silence of its length. The
//! export reads the same stream. The music of a list is mixed in where the
//! list is read, see `pieces/music.rs`.

mod music;

use std::{
    f64::consts::{FRAC_PI_2, PI},
    ptr::null,
    sync::Arc,
};

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
        VideoPiece, VideoSource,
        audio::{SPEEDS, TAIL, drain, pieces::music::MusicSound, push_samples, tempo_graph},
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

pub(crate) fn samples(seconds: f64) -> usize {
    (seconds.max(0.0) * f64::from(RATE)).round().lossy_convert()
}

fn seconds(samples: usize) -> f64 {
    count_to_f64(u64::try_from(samples).expect("a sample count fits u64")) / f64::from(RATE)
}

/// The sound track of one file, decoded to stereo floats at `RATE`.
pub(crate) struct FileSound {
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
    /// Samples the sound of the file lasts, 0 when the file does not say.
    length:    usize,
}

impl FileSound {
    /// None when the file has no sound track.
    pub(crate) fn open(source: &VideoSource, reads: &Interrupt) -> Result<Option<Self>, Error> {
        let input = source.open(reads)?;
        let Some(stream) = input.streams().best(media::Type::Audio) else {
            return Ok(None);
        };
        let index = stream.index();
        let time_base: f64 = stream.time_base().into();
        let length = if stream.duration() > 0 {
            let ticks: f64 = stream.duration().lossy_convert();
            samples(ticks * time_base)
        } else {
            let micros: f64 = input.duration().max(0).lossy_convert();
            samples(micros / 1_000_000.0)
        };
        let context = codec::context::Context::from_parameters(stream.parameters())?;
        let decoder = context.decoder().audio()?;
        let resampler = resampler(&decoder)?;
        Ok(Some(Self {
            length,
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
    pub(crate) fn seek(&mut self, sample: usize) -> Result<(), Error> {
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
    pub(crate) fn read(&mut self, most: usize) -> Result<Vec<Frame>, Error> {
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
            // A plain wav file names no layout, only a channel count. The
            // resampler was made with the usual layout for that count, and
            // it refuses a frame that says something else.
            if decoded.channel_layout().is_empty() {
                decoded.set_channel_layout(ChannelLayout::default(i32::from(decoded.channels())));
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

/// The sound of a piece at its gain. What goes past full scale is cut flat.
/// A gain of 1 leaves every sample as the file has it.
fn amplify(frames: &mut [Frame], gain: f32) {
    if (gain - 1.0).abs() < f32::EPSILON {
        return;
    }
    for frame in frames {
        frame.left = (frame.left * gain).clamp(-1.0, 1.0);
        frame.right = (frame.right * gain).clamp(-1.0, 1.0);
    }
}

/// Fades the sound of a piece in at its start and out at its end, in a
/// straight line. `frames` start `from` samples into the piece, which is
/// `length` samples long.
fn fade(frames: &mut [Frame], from: usize, length: usize, piece: &VideoPiece) {
    let fade_in = samples(piece.sound_fade_in);
    let fade_out = samples(piece.sound_fade_out);
    if fade_in == 0 && fade_out == 0 {
        return;
    }
    for (offset, frame) in frames.iter_mut().enumerate() {
        let at = from + offset;
        let left = length.saturating_sub(at);
        let mut factor = 1.0_f32;
        if at < fade_in {
            let part: f32 = (seconds(at) / seconds(fade_in)).lossy_convert();
            factor = factor.min(part);
        }
        if left <= fade_out && fade_out > 0 {
            let part: f32 = (seconds(left) / seconds(fade_out)).lossy_convert();
            factor = factor.min(part);
        }
        if factor < 1.0 {
            frame.left *= factor;
            frame.right *= factor;
        }
    }
}

/// The gain of a sound that comes in at sample `at` of `window` samples, on
/// the equal power curve of a crossfade.
fn rise(at: usize, window: usize) -> f32 {
    let part = (seconds(at) / seconds(window.max(1))).clamp(0.0, 1.0);
    (part * FRAC_PI_2).sin().lossy_convert()
}

/// The gain of a sound that goes out at sample `at` of `window` samples.
fn fall(at: usize, window: usize) -> f32 {
    let part = (seconds(at) / seconds(window.max(1))).clamp(0.0, 1.0);
    (part * FRAC_PI_2).cos().lossy_convert()
}

/// A sound that comes in from silence at sample `at` of `window` samples, on
/// a raised cosine. It starts and ends gently, so the sound neither jumps in
/// nor stops short.
fn soft_rise(at: usize, window: usize) -> f32 {
    let part = (seconds(at) / seconds(window.max(1))).clamp(0.0, 1.0);
    ((1.0 - (part * PI).cos()) / 2.0).lossy_convert()
}

/// A sound that goes out to silence at sample `at` of `window` samples.
fn soft_fall(at: usize, window: usize) -> f32 {
    let part = (seconds(at) / seconds(window.max(1))).clamp(0.0, 1.0);
    f64::midpoint(1.0, (part * PI).cos()).lossy_convert()
}

/// Which side of a cut has sound in its file beyond the cut.
#[derive(Clone, Copy)]
struct Sides {
    /// The piece before the cut has sound past its end.
    after:  bool,
    /// The piece after the cut has sound before its start.
    before: bool,
}

/// Multiplies every frame by its gain.
fn shape(frames: &mut [Frame], gain: impl Fn(usize) -> f32) {
    for (index, frame) in frames.iter_mut().enumerate() {
        let gain = gain(index);
        frame.left *= gain;
        frame.right *= gain;
    }
}

/// Adds `theirs` to `frames`, each at its gain, cut flat at full scale.
fn mix(frames: &mut [Frame], theirs: &[Frame], gain: impl Fn(usize) -> f32) {
    for (index, (frame, their)) in frames.iter_mut().zip(theirs).enumerate() {
        let gain = gain(index);
        frame.left = (frame.left + their.left * gain).clamp(-1.0, 1.0);
        frame.right = (frame.right + their.right * gain).clamp(-1.0, 1.0);
    }
}

/// A sample count as a signed number, for a place before the start of a
/// file.
fn signed(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
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
    /// Half the crossfade of every cut in samples, the cut after piece `i`
    /// is entry `i`. 0 where the next piece goes on in the same file.
    halves: Vec<usize>,
    /// Which side of every cut has sound beyond it, found on the first
    /// read near the cut.
    sides:  Vec<Option<Sides>>,
    /// The sample of the list the next read gives.
    at:     usize,
    open:   Option<Open>,
    /// The file of the piece on the other side of a cut, read while the 2
    /// sounds overlap.
    other:  Option<Open>,
    /// The file of the music of the list, not opened for a list with none.
    music:  MusicSound,
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
        let halves = list
            .pieces()
            .windows(2)
            .map(|pair| {
                let (this, next) = (&pair[0], &pair[1]);
                // The sound goes on with no seam there, an overlap would
                // only add the same sound to itself.
                let goes_on = this.source.same_as(&next.source) && (next.start - this.end).abs() < 1e-6;
                if goes_on {
                    0
                } else {
                    samples((this.crossfade / 2.0).min(this.duration() / 2.0).min(next.duration() / 2.0))
                }
            })
            .collect();
        let sides = vec![None; list.pieces().len().saturating_sub(1)];
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
            halves,
            sides,
            at: 0,
            open: None,
            other: None,
            music: MusicSound::default(),
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
        let length = self.starts[piece + 1] - self.starts[piece];
        let inside = self.at - self.starts[piece];
        let head_cut = piece.checked_sub(1);
        let tail_cut = (piece + 1 < self.list.pieces().len()).then_some(piece);
        let head_sides = head_cut.map(|cut| self.cut_sides(cut)).transpose()?;
        let tail_sides = tail_cut.map(|cut| self.cut_sides(cut)).transpose()?;
        let head_half = head_cut.map_or(0, |cut| self.halves[cut]);
        let tail_half = tail_cut.map_or(0, |cut| self.halves[cut]);
        // A side that overlaps fades over half the crossfade on each side
        // of the cut. A side with no sound beyond the cut fades over the
        // whole crossfade on its own side, so it does not drop off.
        let own = |half: usize, beyond: bool| {
            if half == 0 {
                0
            } else if beyond {
                half
            } else {
                (2 * half).min(length / 2)
            }
        };
        let head = head_sides.map_or(0, |sides| own(head_half, sides.before));
        let tail = tail_sides.map_or(0, |sides| own(tail_half, sides.after));
        let tail_from = length - tail;
        // A read stays in 1 part of the piece, so 1 rule holds for all of
        // it: the head, the middle, the tail, and in a long head or tail the
        // half next to the cut where the other sound overlaps.
        let marks = [
            head_half.min(head),
            head,
            tail_from,
            tail_from + tail.saturating_sub(tail_half),
            length,
        ];
        let part_end = marks.into_iter().filter(|mark| *mark > inside).min().unwrap_or(length);
        let left = part_end - inside;
        let place = samples(self.list.pieces()[piece].start) + self.at - self.starts[piece];
        self.open_at(piece, place)?;

        let mut frames = match self.open.as_mut().and_then(|open| open.file.as_mut()) {
            Some(file) => file.read(left)?,
            // The file has no sound track.
            None => vec![Frame::ZERO; left.min(SILENCE)],
        };
        amplify(&mut frames, self.list.gain(piece));
        fade(&mut frames, inside, length, &self.list.pieces()[piece]);
        let list = Arc::clone(&self.list);
        let pieces = list.pieces();
        if let Some(sides) = head_sides
            && inside < head
        {
            // The cut is at the start of this piece. Its sound fades in from
            // before the cut when its file has sound there, else from the cut
            // on a soft curve.
            shape(&mut frames, |index| {
                let at = inside + index;
                if sides.before {
                    rise(head_half + at, 2 * head_half)
                } else {
                    soft_rise(at, head)
                }
            });
            // The piece before plays on past its end and fades out, when its
            // file has the sound for it.
            if sides.after && inside < head_half {
                let end = samples(pieces[piece - 1].end);
                let theirs = self.read_other(piece - 1, signed(end + inside), frames.len())?;
                let gain = list.gain(piece - 1);
                mix(&mut frames, &theirs, |index| {
                    gain * fall(head_half + inside + index, 2 * head_half)
                });
            }
        } else if let Some(sides) = tail_sides
            && tail > 0
            && inside >= tail_from
        {
            // The cut is at the end of this piece. Its sound fades out on
            // past the cut when its file has sound there, else up to the cut
            // on a soft curve.
            let into = inside - tail_from;
            shape(&mut frames, |index| {
                let at = into + index;
                if sides.after {
                    fall(at, 2 * tail_half)
                } else {
                    soft_fall(at, tail)
                }
            });
            // The next piece starts before its start and fades in over the
            // last half of the crossfade before the cut, when its file has
            // the sound for it.
            let early_from = tail - tail_half;
            if sides.before && into >= early_from {
                let along = into - early_from;
                let next_start = samples(pieces[piece + 1].start);
                let place = signed(next_start - tail_half + along);
                let theirs = self.read_other(piece + 1, place, frames.len())?;
                let gain = list.gain(piece + 1);
                mix(&mut frames, &theirs, |index| {
                    gain * rise(along + index, 2 * tail_half)
                });
            }
        }
        if let Some(music) = self.list.music() {
            let total = self.starts[self.starts.len() - 1];
            self.music.mix(music, &mut frames, self.at, total, &self.reads)?;
        }
        self.at += frames.len();
        Ok(frames)
    }

    /// Which side of the cut after a piece has sound beyond the cut, read
    /// once per cut. The file of the piece before is opened for its length.
    fn cut_sides(&mut self, cut: usize) -> Result<Sides, Error> {
        if let Some(sides) = self.sides[cut] {
            return Ok(sides);
        }
        let half = self.halves[cut];
        let list = Arc::clone(&self.list);
        let (this, next) = (&list.pieces()[cut], &list.pieces()[cut + 1]);
        let after = half > 0
            && FileSound::open(&this.source, &self.reads)?
                .is_some_and(|file| file.length >= samples(this.end) + half);
        let sides = Sides {
            after,
            before: half > 0 && samples(next.start) >= half,
        };
        self.sides[cut] = Some(sides);
        Ok(sides)
    }

    /// `count` frames of the file of a piece from its sample `place` on. A
    /// place before the start of the file, or past its end, is silence.
    fn read_other(&mut self, piece: usize, place: i64, count: usize) -> Result<Vec<Frame>, Error> {
        let list = Arc::clone(&self.list);
        let source = &list.pieces()[piece].source;
        let mut file = match self.other.take() {
            Some(open) if open.piece == piece => open.file,
            Some(Open { file: Some(file), .. }) if file.source.same_as(source) => Some(file),
            _ => FileSound::open(source, &self.reads)?,
        };
        let silent = usize::try_from(place.saturating_neg()).unwrap_or(0).min(count);
        let mut frames = vec![Frame::ZERO; silent];
        if let Some(file) = &mut file
            && count > 0
        {
            let start = usize::try_from(place.max(0)).unwrap_or(0);
            if file.at != start {
                file.seek(start)?;
            }
            while frames.len() < count {
                frames.extend(file.read(count - frames.len())?);
            }
        }
        frames.resize(count, Frame::ZERO);
        self.other = Some(Open { piece, file });
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

    /// Every piece plays at its own gain, and the next piece is not touched
    /// by the gain of the one before it.
    #[test]
    fn a_piece_plays_at_its_gain() {
        let mut sound = sound(
            [
                ramp(1.0, 1.25).with_gain(2.0),
                ramp(1.0, 1.25).with_gain(0.5),
                ramp(1.0, 1.25),
            ],
            1.0,
        );
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 36_000);
        for (index, frame) in frames.iter().enumerate() {
            let gain = [2.0, 0.5, 1.0][index / 12_000];
            let expected = ramp_at(48_000 + index % 12_000) * gain;
            assert!(
                (frame.left - expected).abs() < 0.000_02 && (frame.right - expected).abs() < 0.000_02,
                "sample {index} is {}, not {expected}",
                frame.left
            );
        }
    }

    /// A piece with sound fades starts in silence, reaches its full sound
    /// after the fade in, and falls to silence at its end. The piece after
    /// it with no fades plays at once at its full sound.
    #[test]
    fn a_piece_fades_its_sound_in_and_out() {
        let mut sound = sound(
            [ramp(1.0, 1.25).with_sound_fades(0.05, 0.1), ramp(1.0, 1.25)],
            1.0,
        );
        let frames = all(&mut sound);
        assert_eq!(frames.len(), 24_000);
        // 0.05 seconds is 2400 samples, 0.1 seconds 4800.
        assert!(
            frames[0].left.abs() < 1e-6,
            "the first sample is {}",
            frames[0].left
        );
        let half_in = ramp_at(48_000 + 1_200) * 0.5;
        assert!(
            (frames[1_200].left - half_in).abs() < 1e-4,
            "{} against {half_in}",
            frames[1_200].left
        );
        assert_ramp(
            &frames[2_400..12_000 - 4_800],
            48_000 + 2_400,
            "between the fades",
        );
        let half_out = ramp_at(48_000 + 12_000 - 2_400) * 0.5;
        assert!((frames[12_000 - 2_400].left - half_out).abs() < 1e-4);
        assert!(frames[11_999].left.abs() < ramp_at(48_000 + 11_999).abs() * 0.001);
        assert_ramp(&frames[12_000..], 48_000, "the piece with no fades");

        // A fade longer than half the piece is cut to half.
        let long = ramp(1.0, 1.25).with_sound_fades(1.0, 1.0);
        assert!((long.sound_fade_in - 0.125).abs() < 1e-9 && (long.sound_fade_out - 0.125).abs() < 1e-9);
    }

    /// 2 pieces from 2 places of a file with a crossfade of 0.1 seconds,
    /// 2400 samples on each side of the cut. The first piece plays on past
    /// its end and fades out, the second starts before its start and fades
    /// in, both at 0.707 at the cut. Outside the overlap each plays alone.
    #[test]
    fn a_crossfade_overlaps_the_sounds_at_a_cut() {
        let mut crossed = sound([ramp(0.25, 0.5).with_crossfade(0.1), ramp(1.0, 1.25)], 1.0);
        let frames = all(&mut crossed);
        assert_eq!(frames.len(), 24_000);
        assert_ramp(&frames[..12_000 - 2_400], 12_000, "the first piece alone");
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let at_cut = half * (ramp_at(48_000) + ramp_at(24_000));
        assert!(
            (frames[12_000].left - at_cut).abs() < 1e-3,
            "{} against {at_cut}",
            frames[12_000].left
        );
        let before_cut = half * (ramp_at(24_000 - 1) + ramp_at(48_000 - 1));
        assert!(
            (frames[11_999].left - before_cut).abs() < 1e-3,
            "{} against {before_cut}",
            frames[11_999].left
        );
        assert_ramp(
            &frames[12_000 + 2_400..],
            48_000 + 2_400,
            "the second piece alone",
        );

        // A piece that ends where its file ends, then one that starts where
        // its file starts: no sound beyond the cut on either side. The
        // first fades out over the whole crossfade up to the cut, 4800
        // samples, and the second fades in over as long from it, both on a
        // raised cosine that is at a half in its middle.
        let mut dipped = sound([ramp(1.75, 2.0).with_crossfade(0.1), ramp(0.0, 0.25)], 1.0);
        let frames = all(&mut dipped);
        assert_eq!(frames.len(), 24_000);
        assert_ramp(&frames[..12_000 - 4_800], 84_000, "the first piece alone");
        let half_out = 0.5 * ramp_at(84_000 + 12_000 - 2_400);
        assert!(
            (frames[12_000 - 2_400].left - half_out).abs() < 1e-3,
            "{}",
            frames[12_000 - 2_400].left
        );
        assert!(
            frames[11_999].left.abs() < 1e-3,
            "the first piece is out at the cut: {}",
            frames[11_999].left
        );
        assert!(
            frames[12_000].left.abs() < 1e-6,
            "the second piece starts silent: {}",
            frames[12_000].left
        );
        let half_in = 0.5 * ramp_at(2_400);
        assert!(
            (frames[12_000 + 2_400].left - half_in).abs() < 1e-3,
            "{}",
            frames[12_000 + 2_400].left
        );
        assert_ramp(&frames[12_000 + 4_800..], 4_800, "the second piece alone");

        // A piece that goes on in the same file gets no overlap.
        let mut played_on = sound([ramp(0.25, 0.5).with_crossfade(0.1), ramp(0.5, 0.75)], 1.0);
        assert_ramp(&all(&mut played_on), 12_000, "1 file played on");
    }

    /// The end of the fixture is 0.8 loud. At a gain of 2 the part over half
    /// of full scale is cut flat at full scale, on both sides of zero.
    #[test]
    fn a_gain_past_full_scale_is_cut_flat() {
        let mut sound = sound(
            [ramp(0.0, 0.25).with_gain(2.0), ramp(1.5, 2.0).with_gain(2.0)],
            1.0,
        );
        let frames = all(&mut sound);
        assert!(frames.iter().all(|frame| frame.left.abs() <= 1.0 && frame.right.abs() <= 1.0));
        // The first piece goes from -0.8 to -0.6, all of it is past -1.
        assert!(
            frames[..12_000]
                .iter()
                .all(|frame| frame.left.to_bits() == (-1.0_f32).to_bits())
        );
        // The second goes from 0.4 to 0.8. It reaches 0.5, full scale at
        // this gain, after an eighth of a second, 6000 samples.
        assert!((frames[12_000].left - 0.8).abs() < 0.001);
        assert!(frames[12_000 + 5_900].left < 1.0);
        assert!(
            frames[12_000 + 6_100..]
                .iter()
                .all(|frame| frame.left.to_bits() == 1.0_f32.to_bits())
        );
    }

    /// A gain that changes while the list is read is in the very next read.
    #[test]
    fn a_new_gain_is_heard_in_the_next_read() {
        let mut sound = sound([ramp(1.0, 2.0)], 1.0);
        let before = sound.read_list().expect("the fixture decodes");
        assert_ramp(&before, 48_000, "before the change");

        let louder = VideoSource::from_pieces([ramp(1.0, 2.0).with_gain(0.5)]);
        assert!(sound.list.take_gains(louder.piece_list().expect("a list of pieces")));
        let after = sound.read_list().expect("the fixture decodes");
        let expected = ramp_at(48_000 + before.len()) * 0.5;
        assert!(
            (after[0].left - expected).abs() < 0.000_02,
            "{} is not {expected}",
            after[0].left
        );
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
