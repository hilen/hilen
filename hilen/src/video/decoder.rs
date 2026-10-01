//! The video decode thread. It demuxes and decodes a few frames ahead into a
//! bounded queue, on the hardware device when the codec allows it, and hands
//! over NV12 planes ready for `write_texture`. A seek bumps the generation, so
//! frames decoded before it are told apart from the ones after and dropped.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, RecvError, SyncSender, TryRecvError},
    },
    thread::Builder,
};

use ffmpeg_next::{
    Error, Packet, codec, color, decoder,
    format::{Pixel, context::Input},
    frame, media,
    software::scaling,
    threading,
    util::error::EAGAIN,
};
use log::warn;

use crate::{
    gm::LossyConvert,
    video::{
        VideoSource,
        audio::AudioDecoder,
        count_to_f64, hw,
        subtitles::{Cue, CueDecoder},
        tracks::{AudioTrack, SubtitleTrack, audio_tracks, subtitle_tracks},
    },
};

/// Frames decoded ahead of the picture. Small on purpose, a 4K frame is 12 MB.
pub(crate) const QUEUE: usize = 3;

/// The matrix that turns the frame's YUV into RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Matrix {
    Bt709,
    Bt601,
    Bt2020,
}

/// How the frame's signal maps to light. PQ and HLG are the HDR curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Transfer {
    Sdr,
    Pq,
    Hlg,
}

/// One decoded picture, a luma plane and an interleaved chroma plane at half
/// size, each row `stride` bytes as the decoder laid it out. NV12 with a byte
/// per sample, or P010 with 2 bytes per sample when `ten_bit`.
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
    pub ten_bit:    bool,
    pub full_range: bool,
    pub matrix:     Matrix,
    pub transfer:   Transfer,
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
    pub tracks:     Tracks,
}

/// The tracks of a source the app can pick from.
#[derive(Default)]
pub(crate) struct Tracks {
    pub audio:     Vec<AudioTrack>,
    pub subtitles: Vec<SubtitleTrack>,
}

pub(crate) enum Message {
    /// Boxed, it is several times the size of a frame message.
    Info(Box<MediaInfo>),
    Frame(VideoFrame),
    Cue(Cue),
    Eof {
        generation: u32,
    },
    Error(String),
}

pub(crate) enum Command {
    Seek {
        generation: u32,
        seconds:    f64,
    },
    /// The subtitle stream to decode cues of, or none.
    Subtitle(Option<usize>),
    Stop,
}

pub(crate) fn spawn(
    source: VideoSource,
    commands: Receiver<Command>,
    messages: SyncSender<Message>,
    decoded: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
) {
    Builder::new()
        .name("hilen-video".into())
        .spawn(move || {
            if let Err(err) = run(&source, &commands, &messages, &decoded, &stop)
                && messages.send(Message::Error(err.to_string())).is_err()
            {
                // The player is gone, nobody is left to show the error.
                warn!("video {}: {err}", source.location());
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
    cues:       Option<CueDecoder>,
}

/// Opens the source and its decoders, and describes the stream.
fn open(source: &VideoSource, stop: &Arc<AtomicBool>) -> Result<(Decoding, MediaInfo), Error> {
    let input = source.open(stop)?;
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

    let tracks = Tracks {
        audio:     audio_tracks(&input),
        subtitles: subtitle_tracks(&input),
    };

    let audio = match AudioDecoder::open(source, stop, None, 1.0) {
        Ok(audio) => audio,
        Err(err) => {
            warn!("video {}: no sound, {err}", source.location());
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
        tracks,
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
        cues: None,
    };
    Ok((decoding, info))
}

fn run(
    source: &VideoSource,
    commands: &Receiver<Command>,
    messages: &SyncSender<Message>,
    counter: &AtomicU64,
    stop: &Arc<AtomicBool>,
) -> Result<(), Error> {
    let (mut decoding, info) = open(source, stop)?;
    if messages.send(Message::Info(Box::new(info))).is_err() {
        return Ok(());
    }
    let mut eof = false;

    loop {
        // Every queued command, the latest seek wins.
        loop {
            match commands.try_recv() {
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return Ok(()),
                Ok(command) => decoding.apply(command, &mut eof)?,
                Err(TryRecvError::Empty) => break,
            }
        }

        if eof {
            // Nothing to decode until a seek, so block instead of spinning.
            match commands.recv() {
                Ok(Command::Stop) | Err(RecvError) => return Ok(()),
                Ok(command) => decoding.apply(command, &mut eof)?,
            }
            continue;
        }

        let mut packet = Packet::empty();
        match packet.read(&mut decoding.input) {
            Ok(()) => {
                if packet.stream() != decoding.stream {
                    if let Some(cue) = decoding.cue(&packet)
                        && messages.send(Message::Cue(cue)).is_err()
                    {
                        return Ok(());
                    }
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
    /// A seek or a subtitle choice. A stop never comes here, the loop ends on
    /// it.
    fn apply(&mut self, command: Command, eof: &mut bool) -> Result<(), Error> {
        match command {
            Command::Seek { generation, seconds } => {
                self.seek(generation, seconds)?;
                *eof = false;
            }
            Command::Subtitle(None) | Command::Stop => self.cues = None,
            Command::Subtitle(Some(index)) => {
                self.cues = match CueDecoder::open(&self.input, index) {
                    Ok(cues) => Some(cues),
                    Err(err) => {
                        warn!("video: no subtitles from track {index}, {err}");
                        None
                    }
                };
            }
        }
        Ok(())
    }

    /// The cue of a packet of the chosen subtitle stream. A line that fails
    /// to decode is skipped, the film goes on without it.
    fn cue(&mut self, packet: &Packet) -> Option<Cue> {
        let cues = self.cues.as_mut().filter(|cues| cues.stream() == packet.stream())?;
        match cues.decode(packet, self.start, self.generation) {
            Ok(cue) => cue,
            Err(err) => {
                warn!("video: a subtitle line did not decode, {err}");
                None
            }
        }
    }

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

        // The 2 layouts the convert pass reads go up as they are. Anything
        // else, planar YUV from a software decoder, goes through swscale once.
        let mut scaled = frame::Video::empty();
        let nv12 = if matches!(source.format(), Pixel::NV12 | Pixel::P010LE) {
            source
        } else {
            self.scale(source, &mut scaled)?;
            &scaled
        };

        let (full_range, matrix, transfer) = color_info(picture);

        Ok(VideoFrame {
            generation: self.generation,
            pts,
            width: nv12.width(),
            height: nv12.height(),
            y: nv12.data(0).to_vec(),
            y_stride: stride(nv12, 0),
            uv: nv12.data(1).to_vec(),
            uv_stride: stride(nv12, 1),
            ten_bit: nv12.format() == Pixel::P010LE,
            full_range,
            matrix,
            transfer,
            hardware,
        })
    }

    /// Software decoders give planar YUV. It becomes NV12, or P010 when the
    /// source has more than 8 bits, so no depth is lost before the tone map.
    /// One converter, rebuilt when the source changes.
    fn scale(&mut self, source: &frame::Video, out: &mut frame::Video) -> Result<(), Error> {
        let fits = self.scaler.as_ref().is_some_and(|scaler| {
            let input = scaler.input();
            input.format == source.format()
                && input.width == source.width()
                && input.height == source.height()
        });
        if !fits {
            let target = if depth(source.format()) > 8 {
                Pixel::P010LE
            } else {
                Pixel::NV12
            };
            self.scaler = Some(scaling::Context::get(
                source.format(),
                source.width(),
                source.height(),
                target,
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

/// Bits per sample of the first component of a pixel format.
fn depth(format: Pixel) -> i32 {
    let Some(descriptor) = format.descriptor() else {
        return 8;
    };
    // SAFETY: ffmpeg's descriptors are static tables, the pointer of an
    // existing one is valid for the life of the program.
    unsafe { (*descriptor.as_ptr()).comp[0].depth }
}

/// Full range, the matrix and the transfer curve, from the stream when it
/// says. Else the usual guess: standard definition is 601, anything bigger
/// 709, and SDR.
fn color_info(picture: &frame::Video) -> (bool, Matrix, Transfer) {
    let full_range = picture.color_range() == color::Range::JPEG;
    let matrix = match picture.color_space() {
        color::Space::BT470BG | color::Space::SMPTE170M | color::Space::SMPTE240M => Matrix::Bt601,
        color::Space::BT709 => Matrix::Bt709,
        color::Space::BT2020NCL | color::Space::BT2020CL => Matrix::Bt2020,
        _ if picture.height() < 720 => Matrix::Bt601,
        _ => Matrix::Bt709,
    };
    let transfer = match picture.color_transfer_characteristic() {
        color::TransferCharacteristic::SMPTE2084 => Transfer::Pq,
        color::TransferCharacteristic::ARIB_STD_B67 => Transfer::Hlg,
        _ => Transfer::Sdr,
    };
    (full_range, matrix, transfer)
}
