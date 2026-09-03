//! The video decode thread. It demuxes and decodes a few frames ahead into a
//! bounded queue, on the hardware device when the codec allows it, and hands
//! over NV12 planes ready for `write_texture`. A seek bumps the generation, so
//! frames decoded before it are told apart from the ones after and dropped.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvError, SyncSender, TryRecvError},
    },
    thread::Builder,
};

use ffmpeg_next::{
    Error, Packet, codec, color, decoder,
    format::{self, Pixel, context::Input},
    frame, media,
    software::scaling,
    threading,
    util::error::EAGAIN,
};
use log::warn;

use crate::{
    gm::LossyConvert,
    video::{audio::AudioDecoder, count_to_f64, hw},
};

/// Frames decoded ahead of the picture. Small on purpose, a 4K frame is 12 MB.
pub(crate) const QUEUE: usize = 3;

/// One decoded picture as NV12, a luma plane and an interleaved chroma plane
/// at half size, each row `stride` bytes as the decoder laid it out.
pub(crate) struct VideoFrame {
    pub generation: u32,
    /// Seconds from the start of the stream.
    pub pts:        f64,
    pub width:      u32,
    pub height:     u32,
    pub y:          Vec<u8>,
    pub y_stride:   u32,
    pub uv:         Vec<u8>,
    pub uv_stride:  u32,
    pub full_range: bool,
    pub bt601:      bool,
    /// Decoded by the hardware device, not the software codec.
    pub hardware:   bool,
}

pub(crate) struct MediaInfo {
    pub duration:   f64,
    pub width:      u32,
    pub height:     u32,
    pub frame_rate: f64,
    pub decoder:    String,
    pub audio:      Option<AudioDecoder>,
}

pub(crate) enum Message {
    Info(MediaInfo),
    Frame(VideoFrame),
    Eof { generation: u32 },
    Error(String),
}

pub(crate) enum Command {
    Seek { generation: u32, seconds: f64 },
    Stop,
}

pub(crate) fn spawn(
    source: String,
    commands: Receiver<Command>,
    messages: SyncSender<Message>,
    decoded: Arc<AtomicU64>,
) {
    Builder::new()
        .name("hilen-video".into())
        .spawn(move || {
            if let Err(err) = run(&source, &commands, &messages, &decoded)
                && messages.send(Message::Error(err.to_string())).is_err()
            {
                // The player is gone, nobody is left to show the error.
                warn!("video {source}: {err}");
            }
        })
        .expect("failed to spawn the video decode thread");
}

struct Decoding {
    input:      Input,
    stream:     usize,
    time_base:  f64,
    /// Seconds the stream's first timestamp sits at, taken off every pts.
    start:      f64,
    frame_rate: f64,
    decoder:    decoder::Video,
    scaler:     Option<scaling::Context>,
    generation: u32,
    /// Seconds a seek asked for. Decoding restarts at the keyframe before it,
    /// so the frames up to here are decoded and dropped.
    skip_until: Option<f64>,
    sent:       u64,
}

/// Opens the source and its decoders, and describes the stream.
fn open(source: &str) -> Result<(Decoding, MediaInfo), Error> {
    crate::video::init();

    let input = format::input(source)?;
    let stream = input.streams().best(media::Type::Video).ok_or(Error::StreamNotFound)?;
    let index = stream.index();
    let time_base: f64 = stream.time_base().into();
    let start = if stream.start_time() > 0 {
        let first: f64 = stream.start_time().lossy_convert();
        first * time_base
    } else {
        0.0
    };
    let rate: f64 = stream.avg_frame_rate().into();
    let frame_rate = if rate.is_finite() && rate > 0.0 {
        rate
    } else {
        30.0
    };
    let duration = if input.duration() > 0 {
        let micros: f64 = input.duration().lossy_convert();
        micros / 1_000_000.0
    } else if stream.duration() > 0 {
        let ticks: f64 = stream.duration().lossy_convert();
        ticks * time_base
    } else {
        0.0
    };

    let mut context = codec::context::Context::from_parameters(stream.parameters())?;
    let mut threads = context.threading();
    threads.kind = threading::Type::Frame;
    threads.count = 0;
    context.set_threading(threads);
    hw::attach(&mut context);
    let decoder = context.decoder().video()?;
    let name = decoder.codec().map(|codec| codec.name().to_string()).unwrap_or_default();

    let audio = match AudioDecoder::open(source) {
        Ok(audio) => audio,
        Err(err) => {
            warn!("video {source}: no sound, {err}");
            None
        }
    };

    let info = MediaInfo {
        duration,
        width: decoder.width(),
        height: decoder.height(),
        frame_rate,
        decoder: name,
        audio,
    };
    let decoding = Decoding {
        input,
        stream: index,
        time_base,
        start,
        frame_rate,
        decoder,
        scaler: None,
        generation: 0,
        skip_until: None,
        sent: 0,
    };
    Ok((decoding, info))
}

fn run(
    source: &str,
    commands: &Receiver<Command>,
    messages: &SyncSender<Message>,
    counter: &AtomicU64,
) -> Result<(), Error> {
    let (mut decoding, info) = open(source)?;
    if messages.send(Message::Info(info)).is_err() {
        return Ok(());
    }
    let mut eof = false;

    loop {
        // Every queued command, the latest seek wins.
        loop {
            match commands.try_recv() {
                Ok(Command::Seek { generation, seconds }) => {
                    decoding.seek(generation, seconds)?;
                    eof = false;
                }
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => break,
            }
        }

        if eof {
            // Nothing to decode until a seek, so block instead of spinning.
            match commands.recv() {
                Ok(Command::Seek { generation, seconds }) => {
                    decoding.seek(generation, seconds)?;
                    eof = false;
                }
                Ok(Command::Stop) | Err(RecvError) => return Ok(()),
            }
            continue;
        }

        let mut packet = Packet::empty();
        match packet.read(&mut decoding.input) {
            Ok(()) => {
                if packet.stream() != decoding.stream {
                    continue;
                }
                decoding.decoder.send_packet(&packet)?;
            }
            Err(Error::Eof) => {
                decoding.decoder.send_eof()?;
                eof = true;
            }
            Err(err) => return Err(err),
        }

        if !decoding.receive(messages, counter)? {
            return Ok(());
        }
        if eof
            && messages
                .send(Message::Eof {
                    generation: decoding.generation,
                })
                .is_err()
        {
            return Ok(());
        }
    }
}

impl Decoding {
    fn seek(&mut self, generation: u32, seconds: f64) -> Result<(), Error> {
        self.generation = generation;
        let target = seconds.max(0.0);
        let micros: i64 = ((target + self.start) * 1_000_000.0).lossy_convert();
        self.input.seek(micros, ..micros)?;
        self.decoder.flush();
        self.skip_until = Some(target);
        Ok(())
    }

    /// Every frame the decoder has ready, converted and sent. False once the
    /// player is gone.
    fn receive(&mut self, messages: &SyncSender<Message>, counter: &AtomicU64) -> Result<bool, Error> {
        loop {
            let mut picture = frame::Video::empty();
            match self.decoder.receive_frame(&mut picture) {
                Ok(()) => {}
                Err(Error::Other { errno: EAGAIN } | Error::Eof) => return Ok(true),
                Err(err) => return Err(err),
            }

            let pts = self.seconds(&picture);
            if self.skip_until.is_some_and(|until| pts + 0.5 / self.frame_rate < until) {
                continue;
            }
            self.skip_until = None;

            let frame = self.convert(&picture, pts)?;
            self.sent += 1;
            counter.fetch_add(1, Ordering::Relaxed);
            if messages.send(Message::Frame(frame)).is_err() {
                return Ok(false);
            }
        }
    }

    fn seconds(&self, picture: &frame::Video) -> f64 {
        match picture.pts().or(picture.timestamp()) {
            Some(pts) => {
                let ticks: f64 = pts.lossy_convert();
                (ticks * self.time_base - self.start).max(0.0)
            }
            None => count_to_f64(self.sent) / self.frame_rate,
        }
    }

    fn convert(&mut self, picture: &frame::Video, pts: f64) -> Result<VideoFrame, Error> {
        let hardware = picture.format() == hw::pixel();
        let mut transferred = frame::Video::empty();
        let source = if hardware {
            hw::transfer(picture, &mut transferred)?;
            &transferred
        } else {
            picture
        };

        let mut scaled = frame::Video::empty();
        let nv12 = if source.format() == Pixel::NV12 {
            source
        } else {
            self.scale(source, &mut scaled)?;
            &scaled
        };

        let (full_range, bt601) = color_info(picture);

        Ok(VideoFrame {
            generation: self.generation,
            pts,
            width: nv12.width(),
            height: nv12.height(),
            y: nv12.data(0).to_vec(),
            y_stride: stride(nv12, 0),
            uv: nv12.data(1).to_vec(),
            uv_stride: stride(nv12, 1),
            full_range,
            bt601,
            hardware,
        })
    }

    /// Software decoders give planar YUV, 10 bit hardware content comes back
    /// as P010. One converter, rebuilt when the source changes.
    fn scale(&mut self, source: &frame::Video, out: &mut frame::Video) -> Result<(), Error> {
        let fits = self.scaler.as_ref().is_some_and(|scaler| {
            let input = scaler.input();
            input.format == source.format()
                && input.width == source.width()
                && input.height == source.height()
        });
        if !fits {
            self.scaler = Some(scaling::Context::get(
                source.format(),
                source.width(),
                source.height(),
                Pixel::NV12,
                source.width(),
                source.height(),
                scaling::Flags::BILINEAR,
            )?);
        }
        self.scaler.as_mut().expect("the scaler was just made").run(source, out)
    }
}

fn stride(picture: &frame::Video, plane: usize) -> u32 {
    u32::try_from(picture.stride(plane)).expect("a plane stride fits u32")
}

/// Full range and the BT.601 matrix, from the stream when it says, else the
/// usual guess: standard definition is 601, anything bigger 709.
fn color_info(picture: &frame::Video) -> (bool, bool) {
    let full_range = picture.color_range() == color::Range::JPEG;
    let bt601 = match picture.color_space() {
        color::Space::BT470BG | color::Space::SMPTE170M | color::Space::SMPTE240M => true,
        color::Space::BT709 => false,
        _ => picture.height() < 720,
    };
    (full_range, bt601)
}
