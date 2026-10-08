//! The video decode thread. It demuxes and decodes a few frames ahead into a
//! bounded queue, on the hardware device when the codec allows it, and hands
//! over NV12 planes ready for `write_texture`. A seek bumps the generation, so
//! frames decoded before it are told apart from the ones after and dropped.

pub(crate) mod pieces;
pub(crate) mod reader;

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvError, SyncSender, TryRecvError},
    },
    thread::Builder,
};

use ffmpeg_next::{
    Error, Packet, Stream, codec, color, decoder,
    format::{Pixel, context::Input},
    frame, media,
    software::scaling,
    threading,
    util::error::EAGAIN,
};
use log::{debug, warn};

use crate::{
    audio::manager::audio_manager,
    gm::LossyConvert,
    video::{
        VideoSource,
        audio::AudioDecoder,
        count_to_f64, hw,
        source::{Interrupt, transport_error},
        subtitles::{Cue, CueDecoder},
        tracks::{AudioTrack, SubtitleTrack, audio_tracks, subtitle_tracks},
    },
};

mod reconnect;

use reconnect::mended;

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

/// Why a round of the decode loop stopped.
enum Halt {
    /// A seek broke a read that waited on the network.
    Seek,
    /// A read or a seek of the source failed. A network stream is opened
    /// again after it.
    Source(Error),
    /// The decoder failed, a fresh connection would not help.
    Decoder(Error),
}

impl From<Error> for Halt {
    fn from(err: Error) -> Self {
        Self::Decoder(err)
    }
}

/// What a round of the decode loop did.
enum Round {
    /// A packet or the end of the stream was read.
    Read,
    /// Only a command was taken.
    Command,
    /// The player is gone.
    Gone,
}

/// `reads` ends a read of the picture demuxer that waits on the network.
/// With `sound` the thread also opens the sound decoder and hands it over,
/// a thread that starts again after a failure leaves that to the player.
pub(crate) fn spawn(
    source: VideoSource,
    commands: Receiver<Command>,
    messages: SyncSender<Message>,
    decoded: Arc<AtomicU64>,
    reads: Interrupt,
    sound: bool,
) {
    Builder::new()
        .name("hilen-video".into())
        .spawn(move || {
            if let Err(err) = run(&source, &commands, &messages, &decoded, &reads, sound)
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
    /// Seconds decoding stands at: the target of the last seek, then the
    /// end of the last frame sent.
    at:         f64,
    /// The source was opened again after a broken read and has to be
    /// brought back to these seconds, unless a seek comes first.
    resume:     Option<f64>,
}

/// Opens the source and its picture decoder, and describes the stream. The
/// sound is opened by the caller.
fn open(source: &VideoSource, reads: &Interrupt) -> Result<(Decoding, MediaInfo), Error> {
    let input = source.open(reads)?;
    let stream = input.streams().best(media::Type::Video).ok_or(Error::StreamNotFound)?;
    let index = stream.index();
    let time_base: f64 = stream.time_base().into();
    let start = first_timestamp(&stream);
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
    let name = decoder.codec().map_or_default(|codec| codec.name().to_string());

    let tracks = Tracks {
        audio:     audio_tracks(&input),
        subtitles: subtitle_tracks(&input),
    };

    let info = MediaInfo {
        duration,
        width: decoder.width(),
        height: decoder.height(),
        frame_rate,
        decoder: name,
        audio: None,
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
        at: 0.0,
        resume: None,
    };
    Ok((decoding, info))
}

/// Seconds the first timestamp of a stream sits at.
pub(crate) fn first_timestamp(stream: &Stream) -> f64 {
    if stream.start_time() > 0 {
        let first: f64 = stream.start_time().lossy_convert();
        let time_base: f64 = stream.time_base().into();
        first * time_base
    } else {
        0.0
    }
}

fn run(
    source: &VideoSource,
    commands: &Receiver<Command>,
    messages: &SyncSender<Message>,
    counter: &AtomicU64,
    reads: &Interrupt,
    sound: bool,
) -> Result<(), Error> {
    if let Some(list) = source.piece_list() {
        return pieces::run(list, commands, messages, counter, reads);
    }
    let (mut decoding, mut info) = open(source, reads)?;
    if sound {
        info.audio = match AudioDecoder::open(source, reads.fresh(), None, 1.0) {
            Ok(audio) => audio,
            Err(err) => {
                warn!("video {}: no sound, {err}", source.location());
                None
            }
        };
    }
    // The first sound of the app opens the audio device, about a second on an
    // iOS simulator. Here it costs the first video its start, on the first
    // play it would hold the main thread for that long.
    if info.audio.is_some() {
        drop(audio_manager());
    }
    if messages.send(Message::Info(Box::new(info))).is_err() {
        return Ok(());
    }
    let mut eof = false;
    // A break of the stream, from the read that failed until a packet
    // comes through again.
    let mut fault = None;

    loop {
        let halt = match decoding.step(commands, messages, counter, &mut eof, reads) {
            Ok(Round::Read) => {
                mended(&mut fault, source, decoding.at);
                continue;
            }
            Ok(Round::Command) => continue,
            Ok(Round::Gone) => return Ok(()),
            Err(halt) => halt,
        };
        if reads.stopped() {
            return Ok(());
        }
        let error = match halt {
            Halt::Decoder(err) => return Err(err),
            // A seek broke a read that waited on the network. The
            // connection of that read is of no use, the source opens again.
            Halt::Seek => match decoding.reopen(source, reads) {
                Ok(()) => {
                    eof = false;
                    continue;
                }
                Err(_) if reads.stopped() => return Ok(()),
                Err(err) => err,
            },
            Halt::Source(err) => err,
        };
        // A file that fails to read fails the same way the next time.
        if !source.is_network() {
            return Err(error);
        }
        if !decoding.reconnect(source, reads, &mut fault, error)? {
            return Ok(());
        }
        eof = false;
    }
}

impl Decoding {
    /// One round of the decode loop: the queued commands, then one packet.
    fn step(
        &mut self,
        commands: &Receiver<Command>,
        messages: &SyncSender<Message>,
        counter: &AtomicU64,
        eof: &mut bool,
        reads: &Interrupt,
    ) -> Result<Round, Halt> {
        // Every queued command, the latest seek wins.
        loop {
            match commands.try_recv() {
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return Ok(Round::Gone),
                Ok(command) => self.apply(command, eof, reads)?,
                Err(TryRecvError::Empty) => break,
            }
        }
        // After a reopen with no seek in the queue decoding goes on where
        // it stood.
        if let Some(at) = self.resume.take() {
            self.seek(self.generation, at, reads)?;
        }

        if *eof {
            // Nothing to decode until a seek, so block instead of spinning.
            match commands.recv() {
                Ok(Command::Stop) | Err(RecvError) => return Ok(Round::Gone),
                Ok(command) => self.apply(command, eof, reads)?,
            }
            return Ok(Round::Command);
        }

        let mut packet = Packet::empty();
        let read = {
            let _reading = reads.reading();
            packet.read(&mut self.input)
        };
        match read {
            Ok(()) => {
                // A packet came, and still the connection may have broken
                // on the way to it, with the packets up to this one lost.
                if let Some(err) = transport_error(&self.input) {
                    // A read a seek broke leaves its error on record too.
                    return Err(if reads.is_broken() {
                        Halt::Seek
                    } else {
                        Halt::Source(err)
                    });
                }
                if packet.stream() != self.stream {
                    if let Some(cue) = self.cue(&packet)
                        && messages.send(Message::Cue(cue)).is_err()
                    {
                        return Ok(Round::Gone);
                    }
                    return Ok(Round::Read);
                }
                self.decoder.send_packet(&packet)?;
            }
            // Some demuxers report a broken read as the end of the file.
            Err(_) if reads.is_broken() => return Err(Halt::Seek),
            Err(Error::Eof) => {
                // And some report a stream that was cut that way too.
                if let Some(err) = transport_error(&self.input) {
                    return Err(Halt::Source(err));
                }
                self.decoder.send_eof()?;
                *eof = true;
            }
            Err(err) => return Err(Halt::Source(err)),
        }

        if !self.receive(messages, counter)? {
            return Ok(Round::Gone);
        }
        if *eof
            && messages
                .send(Message::Eof {
                    generation: self.generation,
                })
                .is_err()
        {
            return Ok(Round::Gone);
        }
        Ok(Round::Read)
    }

    /// Opens the source again after a broken read, with the generation, the
    /// subtitle track and the place decoding stood at carried over. A seek
    /// that waits in the queue is taken by the next step before anything is
    /// read at the old place.
    fn reopen(&mut self, source: &VideoSource, reads: &Interrupt) -> Result<(), Error> {
        loop {
            reads.clear();
            match open(source, reads) {
                Ok((mut fresh, _)) => {
                    fresh.generation = self.generation;
                    fresh.sent = self.sent;
                    fresh.at = self.at;
                    fresh.resume = Some(self.at);
                    fresh.cues = self.cues.as_ref().and_then(|cues| fresh.open_cues(cues.stream()));
                    debug!(
                        "video {}: open again after a broken read, decoding stood at {:.2} s",
                        source.location(),
                        self.at
                    );
                    *self = fresh;
                    return Ok(());
                }
                // Another seek came while it opened.
                Err(_) if reads.is_broken() && !reads.stopped() => {}
                Err(err) => return Err(err),
            }
        }
    }

    fn open_cues(&self, index: usize) -> Option<CueDecoder> {
        match CueDecoder::open(&self.input, index) {
            Ok(cues) => Some(cues),
            Err(err) => {
                warn!("video: no subtitles from track {index}, {err}");
                None
            }
        }
    }

    /// A seek or a subtitle choice. A stop never comes here, the loop ends on
    /// it.
    fn apply(&mut self, command: Command, eof: &mut bool, reads: &Interrupt) -> Result<(), Halt> {
        match command {
            Command::Seek { generation, seconds } => {
                // Before the seek runs, so a break in the middle of it
                // still leaves a place to come back to.
                self.generation = generation;
                self.at = seconds.max(0.0);
                self.resume = None;
                self.seek(generation, seconds, reads)?;
                debug!("video: seek {generation} to {seconds:.2} s done");
                *eof = false;
            }
            Command::Subtitle(None) | Command::Stop => self.cues = None,
            Command::Subtitle(Some(index)) => self.cues = self.open_cues(index),
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

    fn seek(&mut self, generation: u32, seconds: f64, reads: &Interrupt) -> Result<(), Halt> {
        self.generation = generation;
        let target = seconds.max(0.0);
        self.at = target;
        let micros: i64 = ((target + self.start) * 1_000_000.0).lossy_convert();
        let sought = {
            let _reading = reads.reading();
            self.input.seek(micros, ..micros)
        };
        match sought {
            Ok(()) => {}
            Err(_) if reads.is_broken() => return Err(Halt::Seek),
            Err(err) => return Err(Halt::Source(err)),
        }
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
            self.at = pts + 1.0 / self.frame_rate;
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
