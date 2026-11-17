//! The mp4 file of an export: the h264 encoder, the AAC encoder and the
//! muxer. Pictures and sound go in, in the order of time, packets are woven
//! into the file by their timestamps.

use std::{mem::take, path::Path};

use ffmpeg_next::{
    ChannelLayout, Codec, Dictionary, Error, Packet, Rational, codec, color, encoder,
    format::{self, Pixel, Sample, context::Output, sample::Type},
    frame, picture,
    util::error::EAGAIN,
};
use kira::Frame;
use log::{debug, info};

use crate::{
    gm::LossyConvert,
    video::{
        audio::pieces::RATE,
        decoder::{Matrix, reader::PictureColor},
        export::VideoExportSettings,
    },
};

/// The h264 encoders an archive can hold, the hardware one of each system
/// first. `docs/video.md` says which archive has which.
const H264_ENCODERS: [&str; 4] = ["h264_videotoolbox", "h264_mf", "libx264", "libopenh264"];

/// The pixel format every encoder in `H264_ENCODERS` takes.
pub(super) const PIXEL: Pixel = Pixel::YUV420P;

struct Sound {
    encoder: encoder::Audio,
    stream:  usize,
    /// Frames one packet of the encoder takes.
    chunk:   usize,
    /// Sound that waits for a full chunk.
    waiting: Vec<Frame>,
    /// Samples handed to the encoder so far, the timestamp of the next
    /// chunk.
    sent:    i64,
}

pub(super) struct Writer {
    output:       Output,
    video:        encoder::Video,
    /// The time base of the picture encoder, one frame.
    frame_base:   Rational,
    video_stream: usize,
    sound:        Option<Sound>,
}

/// The first h264 encoder of the archive that opens with these settings.
fn open_h264(
    settings: &VideoExportSettings,
    color: PictureColor,
    rate: Rational,
    global_header: bool,
) -> Result<(Codec, encoder::Video), Error> {
    let mut last = Error::EncoderNotFound;
    for name in H264_ENCODERS {
        let Some(codec) = encoder::find_by_name(name) else {
            continue;
        };
        let mut video = codec::context::Context::new_with_codec(codec).encoder().video()?;
        video.set_width(settings.width);
        video.set_height(settings.height);
        video.set_format(PIXEL);
        video.set_time_base(rate.invert());
        video.set_frame_rate(Some(rate));
        video.set_bit_rate(settings.video_bit_rate);
        // A keyframe every 2 seconds, so a player seeks without a long wait.
        video.set_gop((settings.frame_rate * 2.0).round().max(1.0).lossy_convert());
        video.set_colorspace(match color.matrix {
            Matrix::Bt709 => color::Space::BT709,
            Matrix::Bt601 => color::Space::SMPTE170M,
            Matrix::Bt2020 => color::Space::BT2020NCL,
        });
        video.set_color_range(if color.full_range {
            color::Range::JPEG
        } else {
            color::Range::MPEG
        });
        if global_header {
            video.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        // VideoToolbox may fall back to its software encoder on a machine
        // with no hardware one. The other encoders do not know the option
        // and leave it.
        let mut options = Dictionary::new();
        options.set("allow_sw", "1");
        match video.open_as_with(codec, options) {
            Ok(opened) => {
                info!("video export: h264 encoder {name}");
                return Ok((codec, opened));
            }
            Err(err) => {
                debug!("video export: encoder {name} does not open, {err}");
                last = err;
            }
        }
    }
    Err(last)
}

fn open_aac(bit_rate: usize, global_header: bool) -> Result<(Codec, encoder::Audio), Error> {
    let codec = encoder::find_by_name("aac").ok_or(Error::EncoderNotFound)?;
    let mut audio = codec::context::Context::new_with_codec(codec).encoder().audio()?;
    audio.set_rate(i32::try_from(RATE).expect("the sample rate fits i32"));
    audio.set_channel_layout(ChannelLayout::STEREO);
    audio.set_format(Sample::F32(Type::Planar));
    audio.set_bit_rate(bit_rate);
    audio.set_time_base(sample_base());
    if global_header {
        audio.set_flags(codec::Flags::GLOBAL_HEADER);
    }
    Ok((codec, audio.open_as(codec)?))
}

/// The time base of the sound encoder, one sample.
fn sample_base() -> Rational {
    Rational::new(1, i32::try_from(RATE).expect("the sample rate fits i32"))
}

impl Writer {
    /// Makes the file and writes its header. `color` says how the samples
    /// of the pictures that will come are to be read, `with_sound` adds the
    /// AAC track.
    pub(super) fn create(
        path: &Path,
        settings: &VideoExportSettings,
        color: PictureColor,
        with_sound: bool,
    ) -> Result<Self, Error> {
        let mut output = format::output_as(path, "mp4")?;
        let global_header = output.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let rate = Rational::from(settings.frame_rate);

        let (codec, video) = open_h264(settings, color, rate, global_header)?;
        let mut stream = output.add_stream(codec)?;
        stream.set_parameters(&video);
        stream.set_time_base(rate.invert());
        stream.set_avg_frame_rate(rate);
        let video_stream = stream.index();

        let sound = if with_sound {
            let (codec, encoder) = open_aac(settings.audio_bit_rate, global_header)?;
            let mut stream = output.add_stream(codec)?;
            stream.set_parameters(&encoder);
            stream.set_time_base(sample_base());
            let chunk = usize::try_from(encoder.frame_size()).expect("a frame size fits usize").max(1);
            Some(Sound {
                encoder,
                stream: stream.index(),
                chunk,
                waiting: Vec::new(),
                sent: 0,
            })
        } else {
            None
        };

        output.write_header()?;
        Ok(Self {
            output,
            video,
            frame_base: rate.invert(),
            video_stream,
            sound,
        })
    }

    /// One frame of the file. `image` is in `PIXEL` at the size of the
    /// settings, `number` counts the frames from 0.
    pub(super) fn write_picture(&mut self, image: &mut frame::Video, number: i64) -> Result<(), Error> {
        image.set_pts(Some(number));
        // The encoder picks the kind of every frame itself.
        image.set_kind(picture::Type::None);
        self.video.send_frame(image)?;
        self.drain_pictures()
    }

    /// More sound of the file, it follows what was written before.
    pub(super) fn write_sound(&mut self, frames: &[Frame]) -> Result<(), Error> {
        let Some(sound) = &mut self.sound else {
            return Ok(());
        };
        sound.waiting.extend_from_slice(frames);
        while sound.waiting.len() >= sound.chunk {
            let chunk: Vec<Frame> = sound.waiting.drain(..sound.chunk).collect();
            sound.send(&chunk)?;
            // An encoder takes the next chunk only once it gave its
            // packets away.
            sound.drain(&mut self.output)?;
        }
        Ok(())
    }

    /// The end of the file: what the encoders still hold and the index.
    pub(super) fn finish(mut self) -> Result<(), Error> {
        self.video.send_eof()?;
        self.drain_pictures()?;
        if let Some(sound) = &mut self.sound {
            // The last chunk is shorter than the others.
            let rest = take(&mut sound.waiting);
            if !rest.is_empty() {
                sound.send(&rest)?;
                sound.drain(&mut self.output)?;
            }
            sound.encoder.send_eof()?;
            sound.drain(&mut self.output)?;
        }
        self.output.write_trailer()
    }

    fn drain_pictures(&mut self) -> Result<(), Error> {
        let (stream, base) = (self.video_stream, self.frame_base);
        let video = &mut self.video;
        write_packets(&mut self.output, stream, base, |packet| {
            video.receive_packet(packet)
        })
    }
}

impl Sound {
    /// The packets the encoder has ready go into the file.
    fn drain(&mut self, output: &mut Output) -> Result<(), Error> {
        let encoder = &mut self.encoder;
        write_packets(output, self.stream, sample_base(), |packet| {
            encoder.receive_packet(packet)
        })
    }

    fn send(&mut self, frames: &[Frame]) -> Result<(), Error> {
        let mut chunk = frame::Audio::new(Sample::F32(Type::Planar), frames.len(), ChannelLayout::STEREO);
        chunk.set_rate(RATE);
        chunk.set_pts(Some(self.sent));
        for (to, from) in chunk.plane_mut::<f32>(0).iter_mut().zip(frames) {
            *to = from.left;
        }
        for (to, from) in chunk.plane_mut::<f32>(1).iter_mut().zip(frames) {
            *to = from.right;
        }
        self.sent += i64::try_from(frames.len()).expect("a chunk length fits i64");
        self.encoder.send_frame(&chunk)
    }
}

/// Every packet an encoder has ready goes into the file, with its
/// timestamps in the time base of its stream.
fn write_packets(
    output: &mut Output,
    stream: usize,
    base: Rational,
    mut receive: impl FnMut(&mut Packet) -> Result<(), Error>,
) -> Result<(), Error> {
    let stream_base = output.stream(stream).ok_or(Error::StreamNotFound)?.time_base();
    loop {
        let mut packet = Packet::empty();
        match receive(&mut packet) {
            Ok(()) => {}
            Err(Error::Other { errno: EAGAIN } | Error::Eof) => return Ok(()),
            Err(err) => return Err(err),
        }
        packet.set_stream(stream);
        packet.rescale_ts(base, stream_base);
        packet.write_interleaved(output)?;
    }
}
