//! The sound track as a kira streaming decoder. It has its own demuxer over
//! the same source, so kira's decode thread pulls sound while the video
//! thread pulls pictures, and kira's playback position is the clock the
//! picture follows.

use std::mem::take;

use ffmpeg_next::{
    ChannelLayout, Error, Packet, codec, decoder,
    format::{self, Sample, context::Input, sample::Type},
    frame, media,
    software::resampling,
    util::error::EAGAIN,
};
use kira::{Frame, sound::streaming::Decoder};

use crate::{gm::LossyConvert, video::count_to_f64};

/// Frames of silence handed out past the end. kira walks chunk by chunk to
/// the sample it wants, an empty chunk would leave it walking forever.
const TAIL: usize = 1024;

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
}

impl AudioDecoder {
    /// None when the source has no sound track.
    pub(crate) fn open(source: &str) -> Result<Option<Self>, Error> {
        crate::video::init();

        let input = format::input(source)?;
        let Some(stream) = input.streams().best(media::Type::Audio) else {
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
        let frames = (duration * f64::from(rate)).ceil().lossy_convert();

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
        }))
    }

    /// The next packet of the sound stream, None at the end.
    fn next_packet(&mut self) -> Result<Option<Packet>, Error> {
        loop {
            let mut packet = Packet::empty();
            match packet.read(&mut self.input) {
                Ok(()) => {
                    if packet.stream() == self.stream {
                        return Ok(Some(packet));
                    }
                }
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
            push_samples(out, &converted);
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
        if !self.pending.is_empty() {
            return Ok(take(&mut self.pending));
        }
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
            }
            self.receive_all(&mut out)?;
        }
        Ok(out)
    }

    /// Lands on the keyframe before the sample and reports where that is.
    /// kira walks forward from there to the sample it asked for.
    fn seek(&mut self, index: usize) -> Result<usize, Error> {
        let seconds =
            count_to_f64(u64::try_from(index).expect("a sample index fits u64")) / f64::from(self.rate);
        let micros: i64 = (seconds * 1_000_000.0).lossy_convert();
        self.input.seek(micros, ..micros)?;
        self.decoder.flush();
        self.eof = false;
        self.pending.clear();

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
            let landed = first.unwrap_or(seconds) * f64::from(self.rate);
            return Ok(landed.round().max(0.0).lossy_convert());
        }
    }
}
