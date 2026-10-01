//! The sound track as a kira streaming decoder. It has its own demuxer over
//! the same source, so kira's decode thread pulls sound while the video
//! thread pulls pictures, and kira's playback position is the clock the
//! picture follows.

use std::mem::take;

use ffmpeg_next::{
    ChannelLayout, Error, Packet, codec, decoder, filter,
    format::{Sample, context::Input, sample::Type},
    frame, media,
    software::resampling,
    util::error::EAGAIN,
};
use kira::{Frame, sound::streaming::Decoder};

use crate::{
    gm::LossyConvert,
    video::{VideoSource, count_to_f64, source::Interrupt},
};

/// Frames of silence handed out past the end. kira walks chunk by chunk to
/// the sample it wants, an empty chunk would leave it walking forever.
const TAIL: usize = 1024;

/// The speeds the tempo filter takes in one stage.
pub(crate) const SPEEDS: (f64, f64) = (0.5, 4.0);

pub(crate) struct AudioDecoder {
    input:     Input,
    stream:    usize,
    time_base: f64,
    decoder:   decoder::Audio,
    resampler: resampling::Context,
    rate:      u32,
    frames:    usize,
    /// Decoded past a seek but not handed out yet.
    pending:   Vec<Frame>,
    eof:       bool,
    /// How fast the sound plays, 1 is its own speed. kira sees a sound that
    /// is the media length divided by this, so its sample `n` is the media
    /// sample `n * speed`.
    speed:     f64,
    /// Changes the speed and keeps the pitch, none at speed 1.
    tempo:     Option<filter::Graph>,
    /// Ends a read of this demuxer that waits on the network.
    reads:     Interrupt,
    /// kira asked for its first seek. It runs that one inside `play`, on
    /// the thread that starts the sound, which is the main thread.
    asked:     bool,
    /// The sample of that first seek, until the first `decode` runs it on
    /// kira's decode thread.
    first:     Option<usize>,
}

/// A graph that plays stereo floats at `speed` with the pitch kept, ffmpeg's
/// `atempo`.
fn tempo_graph(rate: u32, speed: f64) -> Result<filter::Graph, Error> {
    let buffer = filter::find("abuffer").ok_or(Error::FilterNotFound)?;
    let sink = filter::find("abuffersink").ok_or(Error::FilterNotFound)?;

    let mut graph = filter::Graph::new();
    let format = format!("time_base=1/{rate}:sample_rate={rate}:sample_fmt=flt:channel_layout=stereo");
    graph.add(&buffer, "in", &format)?;
    graph.add(&sink, "out", "")?;
    graph.output("in", 0)?.input("out", 0)?.parse(&format!("atempo={speed}"))?;
    graph.validate()?;
    Ok(graph)
}

impl AudioDecoder {
    /// None when the source has no sound track.
    /// `track` is the stream to play, the best one when none is named.
    /// `reads` is this demuxer's own, a break of it ends the decoder for
    /// good, it gives silence from then on: kira takes a seek only between
    /// reads, so the player drops a sound that waits in a read and opens a
    /// fresh one.
    pub(crate) fn open(
        source: &VideoSource,
        reads: Interrupt,
        track: Option<usize>,
        speed: f64,
    ) -> Result<Option<Self>, Error> {
        let input = source.open(&reads)?;
        let stream = match track {
            Some(index) => input
                .streams()
                .find(|stream| stream.index() == index && stream.parameters().medium() == media::Type::Audio),
            None => input.streams().best(media::Type::Audio),
        };
        let Some(stream) = stream else {
            return Ok(None);
        };
        let index = stream.index();
        let time_base: f64 = stream.time_base().into();
        let duration = if input.duration() > 0 {
            let micros: f64 = input.duration().lossy_convert();
            micros / 1_000_000.0
        } else {
            let ticks: f64 = stream.duration().max(0).lossy_convert();
            ticks * time_base
        };

        let context = codec::context::Context::from_parameters(stream.parameters())?;
        let decoder = context.decoder().audio()?;
        let rate = decoder.rate();
        let layout = if decoder.channel_layout().is_empty() {
            ChannelLayout::default(i32::from(decoder.channels()))
        } else {
            decoder.channel_layout()
        };
        let resampler = resampling::Context::get(
            decoder.format(),
            layout,
            rate,
            Sample::F32(Type::Packed),
            ChannelLayout::STEREO,
            rate,
        )?;
        let speed = speed.clamp(SPEEDS.0, SPEEDS.1);
        let frames = (duration * f64::from(rate) / speed).ceil().lossy_convert();
        let tempo = if (speed - 1.0).abs() < f64::EPSILON {
            None
        } else {
            Some(tempo_graph(rate, speed)?)
        };

        Ok(Some(Self {
            input,
            stream: index,
            time_base,
            decoder,
            resampler,
            rate,
            frames,
            pending: Vec::new(),
            eof: false,
            speed,
            tempo,
            reads,
            asked: false,
            first: None,
        }))
    }

    /// The interrupt of this demuxer, for the player to watch its reads.
    pub(crate) fn reads(&self) -> Interrupt {
        self.reads.clone()
    }

    /// The index of the stream it plays.
    pub(crate) fn stream(&self) -> usize {
        self.stream
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
                Ok(()) => {
                    if packet.stream() == self.stream {
                        return Ok(Some(packet));
                    }
                }
                // The player broke the read and dropped this sound. It ends
                // here in silence. An error would be worse, kira calls a
                // decoder that fails again and again with no pause.
                Err(_) if self.reads.is_broken() => return Ok(None),
                Err(Error::Eof) => return Ok(None),
                Err(err) => return Err(err),
            }
        }
    }

    /// Every frame the decoder has ready, resampled into `out`. Returns the
    /// timestamp of the first one in seconds.
    fn receive_all(&mut self, out: &mut Vec<Frame>) -> Result<Option<f64>, Error> {
        let mut first = None;
        loop {
            let mut decoded = frame::Audio::empty();
            match self.decoder.receive_frame(&mut decoded) {
                Ok(()) => {}
                Err(Error::Other { errno: EAGAIN } | Error::Eof) => return Ok(first),
                Err(err) => return Err(err),
            }
            if first.is_none()
                && let Some(pts) = decoded.pts().or(decoded.timestamp())
            {
                let ticks: f64 = pts.lossy_convert();
                first = Some(ticks * self.time_base);
            }
            let mut converted = frame::Audio::empty();
            self.resampler.run(&decoded, &mut converted)?;
            match &mut self.tempo {
                Some(tempo) => {
                    converted.set_pts(None);
                    tempo.get("in").ok_or(Error::FilterNotFound)?.source().add(&converted)?;
                    drain(tempo, out)?;
                }
                None => push_samples(out, &converted),
            }
        }
    }

    /// The end of the stream: what the tempo filter still holds comes out.
    fn finish_tempo(&mut self, out: &mut Vec<Frame>) -> Result<(), Error> {
        if let Some(tempo) = &mut self.tempo {
            tempo.get("in").ok_or(Error::FilterNotFound)?.source().flush()?;
            drain(tempo, out)?;
        }
        Ok(())
    }
}

/// Every frame the tempo filter has ready.
fn drain(tempo: &mut filter::Graph, out: &mut Vec<Frame>) -> Result<(), Error> {
    let mut sink = tempo.get("out").ok_or(Error::FilterNotFound)?;
    loop {
        let mut filtered = frame::Audio::empty();
        match sink.sink().frame(&mut filtered) {
            Ok(()) => push_samples(out, &filtered),
            Err(Error::Other { errno: EAGAIN } | Error::Eof) => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

/// The interleaved stereo floats of a resampled frame as kira frames.
fn push_samples(out: &mut Vec<Frame>, converted: &frame::Audio) {
    let count = converted.samples() * 2 * size_of::<f32>();
    let bytes = &converted.data(0)[..count];
    let floats: &[f32] = bytemuck::cast_slice(bytes);
    let (pairs, rest) = floats.as_chunks::<2>();
    debug_assert!(rest.is_empty(), "stereo samples come in pairs");
    out.extend(pairs.iter().map(|[left, right]| Frame::new(*left, *right)));
}

impl Decoder for AudioDecoder {
    type Error = Error;

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn num_frames(&self) -> usize {
        self.frames
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Error> {
        if let Some(index) = self.first.take() {
            self.run_first_seek(index)?;
        }
        if !self.pending.is_empty() {
            return Ok(take(&mut self.pending));
        }
        self.next_chunk()
    }

    /// Lands on the keyframe before the sample and reports where that is.
    /// kira walks forward from there to the sample it asked for. The first
    /// seek reads nothing, it reports the sample itself and the first
    /// `decode` goes there, so starting a sound never waits on the source.
    fn seek(&mut self, index: usize) -> Result<usize, Error> {
        if !self.asked {
            self.asked = true;
            self.first = Some(index);
            return Ok(index);
        }
        self.first = None;
        self.seek_now(index)
    }
}

impl AudioDecoder {
    /// The first seek, on the decode thread. kira was told the decoder
    /// stands at `index`, so the sound in hand has to start exactly there.
    fn run_first_seek(&mut self, index: usize) -> Result<(), Error> {
        let landed = self.seek_now(index)?;
        if self.eof {
            return Ok(());
        }
        if landed >= index {
            // A stream whose sound starts a little after the place asked
            // for. More than a second apart is not a gap to fill.
            let gap = landed - index;
            if usize::try_from(self.rate).is_ok_and(|second| gap <= second) {
                self.pending.splice(0..0, vec![Frame::ZERO; gap]);
            }
            return Ok(());
        }
        let mut skip = index - landed;
        while skip > 0 && !self.eof {
            if self.pending.is_empty() {
                self.pending = self.next_chunk()?;
                continue;
            }
            let dropped = skip.min(self.pending.len());
            self.pending.drain(..dropped);
            skip -= dropped;
        }
        Ok(())
    }

    /// The next frames of the stream, silence past its end.
    fn next_chunk(&mut self) -> Result<Vec<Frame>, Error> {
        let mut out = Vec::new();
        while out.is_empty() {
            if self.eof {
                out.resize(TAIL, Frame::ZERO);
                break;
            }
            if let Some(packet) = self.next_packet()? {
                self.decoder.send_packet(&packet)?;
            } else {
                self.eof = true;
                self.decoder.send_eof()?;
                self.receive_all(&mut out)?;
                self.finish_tempo(&mut out)?;
                continue;
            }
            self.receive_all(&mut out)?;
        }
        Ok(out)
    }

    fn seek_now(&mut self, index: usize) -> Result<usize, Error> {
        // kira counts samples of the sound it hears, the stream is `speed`
        // times that far along.
        let seconds = count_to_f64(u64::try_from(index).expect("a sample index fits u64"))
            / f64::from(self.rate)
            * self.speed;
        let micros: i64 = (seconds * 1_000_000.0).lossy_convert();
        let sought = {
            let _reading = self.reads.reading();
            self.input.seek(micros, ..micros)
        };
        match sought {
            Ok(()) => {}
            // A broken decoder is at its end for good, see `next_packet`.
            Err(_) if self.reads.is_broken() => {
                self.eof = true;
                self.pending.clear();
                return Ok(self.frames);
            }
            Err(err) => return Err(err),
        }
        self.decoder.flush();
        self.eof = false;
        self.pending.clear();
        // The filter holds sound from before the seek, a fresh one does not.
        if self.tempo.is_some() {
            self.tempo = Some(tempo_graph(self.rate, self.speed)?);
        }

        loop {
            let Some(packet) = self.next_packet()? else {
                self.eof = true;
                return Ok(self.frames);
            };
            self.decoder.send_packet(&packet)?;
            let mut out = Vec::new();
            let first = self.receive_all(&mut out)?;
            if out.is_empty() {
                continue;
            }
            self.pending = out;
            let landed = first.unwrap_or(seconds) / self.speed * f64::from(self.rate);
            return Ok(landed.round().max(0.0).lossy_convert());
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use kira::sound::streaming::Decoder;

    use crate::video::{audio::AudioDecoder, count_to_f64, source::Interrupt, test_fixture};

    /// The tone of a track in Hz from the sign changes of its sound, and how
    /// many seconds of sound the track gives, at a playback speed.
    fn heard(track: Option<usize>, speed: f64) -> (f64, f64) {
        let stop = Arc::new(AtomicBool::new(false));
        let mut decoder =
            AudioDecoder::open(&test_fixture("tracks.mkv"), Interrupt::new(&stop), track, speed)
                .expect("the fixture opens")
                .expect("the fixture has sound");
        let mut frames = Vec::new();
        while !decoder.eof {
            frames.extend(decoder.decode().expect("the fixture decodes"));
        }
        let crossings = frames
            .windows(2)
            .filter(|pair| (pair[0].left < 0.0) != (pair[1].left < 0.0))
            .count();
        let seconds = count_to_f64(frames.len() as u64) / f64::from(decoder.sample_rate());
        (count_to_f64(crossings as u64) / 2.0 / seconds, seconds)
    }

    fn tone(track: Option<usize>) -> f64 {
        heard(track, 1.0).0
    }

    /// The fixture is 4 seconds of a 440 Hz tone. At double speed it must be
    /// 2 seconds long and at half speed 8, and still sound at 440 Hz, the
    /// pitch is kept. A plain resample would give 880 and 220 Hz.
    #[test]
    fn speed_changes_the_length_and_keeps_the_pitch() {
        for (speed, length) in [(1.0, 4.0), (2.0, 2.0), (0.5, 8.0), (1.5, 4.0 / 1.5)] {
            let (hz, seconds) = heard(Some(1), speed);
            assert!(
                (seconds - length).abs() < 0.1,
                "at speed {speed} the sound is {seconds} s long"
            );
            assert!((hz - 440.0).abs() < 15.0, "at speed {speed} the tone is {hz} Hz");
        }
    }

    fn fixture() -> AudioDecoder {
        let stop = Arc::new(AtomicBool::new(false));
        AudioDecoder::open(&test_fixture("tracks.mkv"), Interrupt::new(&stop), Some(1), 1.0)
            .expect("the fixture opens")
            .expect("the fixture has sound")
    }

    /// kira runs the first seek of a decoder inside `play`, on the main
    /// thread. That seek must read nothing and report the sample it was
    /// asked for, and the first decode must then give the sound from
    /// exactly that sample, the same one a seek that runs at once walks to.
    #[test]
    fn the_first_seek_waits_for_the_first_decode() {
        const SAMPLE: usize = 70_000;
        const COMPARED: usize = 4_000;

        let mut late = fixture();
        assert_eq!(late.seek(SAMPLE).expect("the first seek"), SAMPLE);
        assert!(late.pending.is_empty(), "the first seek decoded nothing");
        let mut heard_late = Vec::new();
        while heard_late.len() < COMPARED {
            heard_late.extend(late.decode().expect("the fixture decodes"));
        }

        // The second seek of a decoder runs at once and lands before the
        // sample, kira drops the sound up to it.
        let mut at_once = fixture();
        assert_eq!(at_once.seek(0).expect("the first seek"), 0);
        let landed = at_once.seek(SAMPLE).expect("the second seek");
        assert!(landed <= SAMPLE, "landed at {landed}");
        let mut heard_at_once = Vec::new();
        while heard_at_once.len() < SAMPLE - landed + COMPARED {
            heard_at_once.extend(at_once.decode().expect("the fixture decodes"));
        }

        let same = heard_late[..COMPARED]
            .iter()
            .zip(&heard_at_once[SAMPLE - landed..])
            .all(|(a, b)| a.left.to_bits() == b.left.to_bits() && a.right.to_bits() == b.right.to_bits());
        assert!(
            same,
            "the sound after the late seek is not the sound at the sample"
        );
    }

    /// The fixture has a 440 Hz track at stream 1 and a 1760 Hz track at
    /// stream 2. The decoder must play the one it was asked for, and the
    /// first one when none is named.
    #[test]
    fn the_chosen_sound_track_plays() {
        assert!(
            (tone(Some(1)) - 440.0).abs() < 20.0,
            "stream 1 is the 440 Hz tone"
        );
        assert!(
            (tone(Some(2)) - 1760.0).abs() < 40.0,
            "stream 2 is the 1760 Hz tone"
        );
        assert!(
            (tone(None) - 440.0).abs() < 20.0,
            "no choice plays the first track"
        );
    }
}
