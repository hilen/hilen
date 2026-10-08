//! Writes a list of pieces into 1 mp4 file with h264 and AAC, on a thread of
//! its own, with progress, a cancel and an error. It reads the list the way
//! the player does, `decoder/pieces.rs` for the pictures and
//! `audio/pieces.rs` for the sound, so the file holds the frames and the
//! samples the preview showed. Which h264 encoder does the work depends on
//! the ffmpeg archive of the system, see `docs/video.md`.

mod canvas;
mod writer;

use std::{
    fs::remove_file,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread::Builder,
};

use anyhow::{Result, anyhow, bail};
use ffmpeg_next::frame;
use log::{error, info, warn};
use parking_lot::Mutex;

use crate::{
    deps::hreads::on_main,
    gm::{LossyConvert, flat::Size},
    video::{
        VideoPiece, VideoSource,
        audio::pieces::{PiecesSound, RATE},
        count_to_f64,
        decoder::pieces::Pieces,
        export::{canvas::Canvas, writer::Writer},
        source::Interrupt,
    },
};

/// What the file of an export is like.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoExportSettings {
    /// Pixels. An odd number is rounded down, h264 needs even sides. A
    /// piece of another shape is fitted in with black around it.
    pub width:          u32,
    pub height:         u32,
    /// Frames per second of the file, whatever the pieces have.
    pub frame_rate:     f64,
    /// Bits per second.
    pub video_bit_rate: usize,
    pub audio_bit_rate: usize,
}

impl VideoExportSettings {
    /// Settings with a bit rate that fits the size and the frame rate, 6
    /// megabits for 1080p at 30 frames, and 192 kilobits for the sound.
    pub fn new(width: u32, height: u32, frame_rate: f64) -> Self {
        let pixels = f64::from(width) * f64::from(height) * frame_rate;
        Self {
            width,
            height,
            frame_rate,
            video_bit_rate: (pixels * 0.1).max(200_000.0).round().lossy_convert(),
            audio_bit_rate: 192_000,
        }
    }
}

/// What `VideoExport::start` reports, on the main thread. `Progress` comes
/// as the file grows, then exactly 1 of the other 3.
#[derive(Debug, Clone, PartialEq)]
pub enum VideoExportEvent {
    /// The part of the file that is written, 0 to 1.
    Progress(f32),
    /// The file is complete.
    Finished,
    /// `cancel` was called. The file is deleted.
    Cancelled,
    /// The export failed, with the reason. The file is deleted.
    Failed(String),
}

/// An export that runs. Dropping it changes nothing, the export goes on.
pub struct VideoExport {
    cancelled: Arc<AtomicBool>,
    progress:  Arc<AtomicU32>,
}

impl VideoExport {
    /// Starts to write `pieces` into the mp4 file at `path`, on a thread of
    /// its own. `each` gets the events on the main thread. A file that is
    /// already at `path` is replaced.
    pub fn start(
        pieces: impl IntoIterator<Item = VideoPiece>,
        path: impl Into<PathBuf>,
        settings: VideoExportSettings,
        each: impl FnMut(VideoExportEvent) + Send + 'static,
    ) -> Self {
        let source = VideoSource::from_pieces(pieces);
        let path = path.into();
        let export = Self {
            cancelled: Arc::default(),
            progress:  Arc::default(),
        };
        let each = Arc::new(Mutex::new(each));
        let report = {
            let each = Arc::clone(&each);
            move |event: VideoExportEvent| {
                let each = Arc::clone(&each);
                on_main(move || (each.lock())(event));
            }
        };

        let (cancelled, progress) = (Arc::clone(&export.cancelled), Arc::clone(&export.progress));
        let failed = report.clone();
        let spawned = Builder::new().name("hilen-video-export".into()).spawn(move || {
            // A new event for every hundredth of the file, not for every
            // frame.
            let mut reported = 0;
            let on_progress = |part: f32| {
                progress.store(part.to_bits(), Ordering::Relaxed);
                let hundredths: u32 = (part * 100.0).floor().lossy_convert();
                if hundredths > reported {
                    reported = hundredths;
                    report(VideoExportEvent::Progress(part));
                }
            };
            let event = match write(&source, &path, &settings, on_progress, &cancelled) {
                Ok(true) => VideoExportEvent::Finished,
                Ok(false) => VideoExportEvent::Cancelled,
                Err(err) => {
                    error!("video export to {}: {err:#}", path.display());
                    VideoExportEvent::Failed(format!("{err:#}"))
                }
            };
            report(event);
        });
        if let Err(err) = spawned {
            error!("video export: no thread to write the file, {err}");
            failed(VideoExportEvent::Failed(err.to_string()));
        }
        export
    }

    /// The part of the file that is written, 0 to 1. Any thread.
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.progress.load(Ordering::Relaxed))
    }

    /// Ends the export. The file is deleted and `Cancelled` is reported.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Writes the file and blocks until it is done. True when the file is
/// complete, false when `cancelled` was set. A file that is not complete is
/// deleted.
fn write(
    source: &VideoSource,
    path: &Path,
    settings: &VideoExportSettings,
    progress: impl FnMut(f32),
    cancelled: &Arc<AtomicBool>,
) -> Result<bool> {
    crate::video::init();
    let done = match encode(source, path, settings, progress, cancelled) {
        // A cancel ends a read that waits with an error. That is the
        // cancel, not a failure.
        Err(_) if cancelled.load(Ordering::Relaxed) => Ok(false),
        done => done,
    };
    if !matches!(done, Ok(true))
        && path.exists()
        && let Err(err) = remove_file(path)
    {
        warn!(
            "video export: {} is not complete and stays, {err}",
            path.display()
        );
    }
    done
}

fn encode(
    source: &VideoSource,
    path: &Path,
    settings: &VideoExportSettings,
    mut progress: impl FnMut(f32),
    cancelled: &Arc<AtomicBool>,
) -> Result<bool> {
    let list = source
        .piece_list()
        .ok_or_else(|| anyhow!("the export takes a list of pieces"))?;
    let size = Size::new(settings.width / 2 * 2, settings.height / 2 * 2);
    if size.width == 0 || size.height == 0 || settings.frame_rate.is_nan() || settings.frame_rate <= 0.0 {
        bail!(
            "an export of {} by {} pixels at {} frames a second is not possible",
            settings.width,
            settings.height,
            settings.frame_rate
        );
    }
    let settings = VideoExportSettings {
        width: size.width,
        height: size.height,
        ..settings.clone()
    };
    // A cancel also ends a read that waits.
    let reads = Interrupt::new(cancelled);
    let stopped = || cancelled.load(Ordering::Relaxed);

    let mut pieces = Pieces::new(list, &reads);
    if pieces.start()?.is_none() {
        bail!("the list has no pieces");
    }
    // The next picture of the list, not due yet.
    let mut upcoming = pieces.next_picture()?;
    let Some(first) = &upcoming else {
        bail!("the list has no picture");
    };
    let with_sound = list.has_sound() == Some(true);
    let mut sound = if with_sound {
        Some(PiecesSound::new(list, 1.0, reads.clone())?)
    } else {
        None
    };
    let mut written = 0;
    let mut writer = Writer::create(path, &settings, first.picture.color(), with_sound)?;
    let mut canvas = Canvas::new(size);

    let frames: u64 = (list.duration() * settings.frame_rate).round().max(1.0).lossy_convert();
    info!(
        "video export to {}: {}, {frames} frames of {} by {} at {} a second",
        path.display(),
        source.location(),
        size.width,
        size.height,
        settings.frame_rate
    );
    // A picture is the one for a frame of the file from a quarter of a
    // frame before its own time. With no such room a picture that is due
    // exactly on a frame would land on either side of it by rounding.
    let early = 0.25 / settings.frame_rate;
    let mut shown: Option<frame::Video> = None;
    for frame in 0..frames {
        if stopped() {
            return Ok(false);
        }
        let time = count_to_f64(frame) / settings.frame_rate;
        // The newest picture that is due by now. The ones before it are
        // left out, only that one is scaled. The first frame of the file
        // takes the first picture, due or not.
        let mut newest = None;
        while let Some(picture) = &upcoming {
            let first = shown.is_none() && newest.is_none();
            if !first && picture.seconds > time + early {
                break;
            }
            newest = upcoming.take();
            upcoming = pieces.next_picture()?;
        }
        if let Some(picture) = newest {
            shown = Some(canvas.frame(&picture.picture.into_software()?)?);
        }
        let Some(image) = &mut shown else {
            bail!("the list has no picture for its first frame");
        };
        writer.write_picture(image, i64::try_from(frame)?)?;

        // The sound up to the end of this frame.
        if let Some(sound) = &mut sound {
            let end = (time + 1.0 / settings.frame_rate).min(list.duration());
            let until: usize = (end * f64::from(RATE)).round().lossy_convert();
            while written < until {
                let frames = sound.read_list()?;
                if frames.is_empty() {
                    break;
                }
                written += frames.len();
                writer.write_sound(&frames)?;
            }
        }
        progress((count_to_f64(frame + 1) / count_to_f64(frames)).lossy_convert());
    }
    writer.finish()?;
    Ok(!stopped())
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use ffmpeg_next::encoder::find_by_name;

    use crate::{
        gm::LossyConvert,
        video::{
            VideoFrames, VideoPicture, VideoPiece, VideoSource,
            audio::pieces::PiecesSound,
            decoder::reader::Reader,
            export::{VideoExportSettings, write},
            source::Interrupt,
            test_fixture,
        },
    };

    /// The prebuilt archive of this system must hold an h264 encoder and
    /// the AAC encoder. This is the check that found the Windows archive
    /// without one.
    #[test]
    fn the_archive_has_the_encoders() {
        crate::video::init();
        #[cfg(macos)]
        let h264 = "h264_videotoolbox";
        #[cfg(win)]
        let h264 = "h264_mf";
        #[cfg(any(macos, win))]
        assert!(
            find_by_name(h264).is_some(),
            "the ffmpeg archive has no {h264} encoder"
        );
        assert!(
            find_by_name("aac").is_some(),
            "the ffmpeg archive has no aac encoder"
        );
    }

    fn ramp(first: u32, end: u32) -> VideoPiece {
        VideoPiece::new(
            test_fixture("ramp.mp4"),
            f64::from(first) / 30.0,
            f64::from(end) / 30.0,
        )
    }

    /// A small file at a bit rate that keeps the grays of the ramp apart.
    fn settings(width: u32, height: u32, frame_rate: f64) -> VideoExportSettings {
        VideoExportSettings {
            video_bit_rate: 3_000_000,
            ..VideoExportSettings::new(width, height, frame_rate)
        }
    }

    fn file(name: &str) -> PathBuf {
        temp_dir().join(format!("hilen-video-export-{name}.mp4"))
    }

    /// Writes the pieces and gives the progress that was reported.
    fn export(
        name: &str,
        pieces: impl IntoIterator<Item = VideoPiece>,
        settings: &VideoExportSettings,
    ) -> Vec<f32> {
        let mut progress = Vec::new();
        let done = write(
            &VideoSource::from_pieces(pieces),
            &file(name),
            settings,
            |part| progress.push(part),
            &Arc::default(),
        );
        assert!(matches!(done, Ok(true)), "the export of {name}: {done:?}");
        progress
    }

    /// The pixel of a picture at a place.
    fn pixel(picture: &VideoPicture, x: u32, y: u32) -> [u8; 3] {
        let at = usize::try_from(y * picture.width + x).expect("a pixel index fits usize") * 4;
        [picture.rgba[at], picture.rgba[at + 1], picture.rgba[at + 2]]
    }

    /// Which frame of the ramp fixture a gray is, see `frames.rs`.
    fn ramp_index(gray: [u8; 3]) -> i32 {
        assert!(
            gray[0].abs_diff(gray[1]) <= 6 && gray[1].abs_diff(gray[2]) <= 6,
            "the ramp is gray, got {gray:?}"
        );
        (f64::from(gray[1]) * 219.0 / 255.0 / 3.0).round().lossy_convert()
    }

    /// Checks that a frame of the file shows this frame of the ramp. The
    /// encoder loses a little, a gray may come out 1 frame of the ramp off,
    /// the frames at a cut are more than 20 apart.
    fn assert_ramp(frames: &mut VideoFrames, frame: u32, rate: f64, expected: i32) {
        let picture = frames.picture(f64::from(frame) / rate, (0, 0)).expect("the file decodes");
        let middle = pixel(&picture, picture.width / 2, picture.height / 2);
        let index = ramp_index(middle);
        assert!(
            (index - expected).abs() <= 1,
            "frame {frame} of the file shows ramp {index}, not {expected}"
        );
    }

    /// How many pictures the file holds.
    fn count_frames(name: &str) -> usize {
        let stop = Arc::new(AtomicBool::new(false));
        let mut reader = Reader::open(
            &VideoSource::new(file(name).to_string_lossy()),
            &Interrupt::new(&stop),
        )
        .expect("the file opens");
        let mut count = 0;
        while reader.next().expect("the file decodes").is_some() {
            count += 1;
        }
        count
    }

    /// 2 pieces of 1 file with a jump between them and a piece of another
    /// file in another shape, written and read back: the length, the frame
    /// count, the frame on each side of both cuts, the black around the
    /// piece of another shape, and the sound on each side of a cut.
    #[test]
    fn a_list_is_written_and_reads_back() {
        let name = "list";
        // The frame of the color fixture at 2 seconds is blue.
        let blue = VideoPiece::new(test_fixture("colors.mp4"), 2.0, 2.5);
        let progress = export(name, [ramp(10, 20), ramp(41, 50), blue], &settings(128, 72, 30.0));
        assert!(progress.is_sorted(), "the progress never goes back");
        assert_eq!(progress.len(), 34);
        assert!(
            (progress[33] - 1.0).abs() < f32::EPSILON,
            "the progress ends at 1"
        );

        let mut frames = VideoFrames::open(file(name).to_string_lossy()).expect("the file opens");
        let info = frames.info().clone();
        assert_eq!((info.width, info.height), (128, 72));
        assert!((info.frame_rate - 30.0).abs() < 0.01, "{}", info.frame_rate);
        // 19 frames of the ramp and half a second of blue.
        let length = 19.0 / 30.0 + 0.5;
        assert!((info.duration - length).abs() < 0.05, "{} s", info.duration);
        assert!(info.has_sound, "the file has a sound track");
        assert_eq!(count_frames(name), 34);

        assert_ramp(&mut frames, 0, 30.0, 10);
        assert_ramp(&mut frames, 9, 30.0, 19);
        assert_ramp(&mut frames, 10, 30.0, 41);
        assert_ramp(&mut frames, 18, 30.0, 49);
        // The square blue picture stands in the middle with black at its
        // sides, from the first frame after the cut to the last frame.
        for frame in [19, 33] {
            let picture = frames.picture(f64::from(frame) / 30.0, (0, 0)).expect("the file decodes");
            let near = |have: [u8; 3], want: [u8; 3]| have.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 16);
            let (middle, side) = (pixel(&picture, 64, 36), pixel(&picture, 8, 36));
            assert!(
                near(middle, [0x34, 0x97, 0xdb]),
                "frame {frame} is {middle:?} in the middle"
            );
            assert!(near(side, [0, 0, 0]), "frame {frame} is {side:?} at the side");
        }

        // The sound of the ramp fixture rises in a line from -0.8 to 0.8
        // over its 2 seconds, so a value tells the place of the file.
        let stop = Arc::new(AtomicBool::new(false));
        let whole = VideoSource::from_pieces([VideoPiece::new(file(name).to_string_lossy(), 0.0, length)]);
        let list = whole.piece_list().expect("a list of pieces");
        let mut sound = PiecesSound::new(list, 1.0, Interrupt::new(&stop)).expect("the sound is made");
        let mut heard = Vec::new();
        loop {
            let chunk = sound.read_list().expect("the file decodes");
            if chunk.is_empty() {
                break;
            }
            heard.extend(chunk);
        }
        let at = |seconds: f64| -> f32 {
            let sample: usize = (seconds * 48_000.0).round().lossy_convert();
            heard[sample].left
        };
        let ramp_at = |seconds: f64| -> f32 { (-0.8 + 0.8 * seconds).lossy_convert() };
        // 0.3 s into the list is 0.3 s into the first piece, which starts
        // at frame 10. The cut is at 0.3333 s, the second piece starts at
        // frame 41. The blue piece has no sound.
        for (seconds, expected) in [
            (0.1, ramp_at(10.0 / 30.0 + 0.1)),
            (0.3, ramp_at(10.0 / 30.0 + 0.3)),
            (0.37, ramp_at(41.0 / 30.0 + 0.37 - 10.0 / 30.0)),
            (0.6, ramp_at(41.0 / 30.0 + 0.6 - 10.0 / 30.0)),
            (0.9, 0.0),
        ] {
            let value = at(seconds);
            assert!(
                (value - expected).abs() < 0.03,
                "the sound at {seconds} s is {value}, not {expected}"
            );
        }
    }

    /// The file has its own frame rate. At half the rate of the piece every
    /// second frame is left out, at double the rate every frame comes
    /// twice, and the length stays. A piece of a wider shape gets black
    /// above and below.
    #[test]
    fn the_file_has_its_own_frame_rate_and_size() {
        export("slow", [ramp(0, 30)], &settings(64, 64, 15.0));
        assert_eq!(count_frames("slow"), 15);
        let mut frames = VideoFrames::open(file("slow").to_string_lossy()).expect("the file opens");
        assert_eq!((frames.info().width, frames.info().height), (64, 64));
        assert!((frames.info().duration - 1.0).abs() < 0.05);
        for frame in [0, 1, 7, 14] {
            assert_ramp(
                &mut frames,
                frame,
                15.0,
                i32::try_from(frame).expect("a small number") * 2,
            );
        }
        let picture = frames.picture(0.5, (0, 0)).expect("the file decodes");
        let bar = pixel(&picture, 32, 4);
        assert!(bar.iter().all(|channel| *channel <= 16), "the bar is {bar:?}");

        export("fast", [ramp(0, 30)], &settings(128, 72, 60.0));
        assert_eq!(count_frames("fast"), 60);
        let mut frames = VideoFrames::open(file("fast").to_string_lossy()).expect("the file opens");
        for frame in [0, 1, 2, 3, 58, 59] {
            assert_ramp(
                &mut frames,
                frame,
                60.0,
                i32::try_from(frame).expect("a small number") / 2,
            );
        }
    }

    #[test]
    fn a_cancelled_export_leaves_no_file() {
        let path = file("cancelled");
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut seen = 0;
        let done = write(
            &VideoSource::from_pieces([ramp(0, 60)]),
            &path,
            &settings(128, 72, 30.0),
            |_| {
                seen += 1;
                if seen == 10 {
                    cancelled.store(true, Ordering::Relaxed);
                }
            },
            &cancelled,
        );
        assert!(matches!(done, Ok(false)), "{done:?}");
        assert_eq!(seen, 10, "nothing is written after the cancel");
        assert!(!path.exists(), "the file of a cancelled export is deleted");
    }

    /// A cancel while the files open ends their reads with an error. The
    /// export then reported that it failed.
    #[test]
    fn a_cancel_before_the_first_frame_is_a_cancel() {
        let path = file("cancelled-early");
        let done = write(
            &VideoSource::from_pieces([ramp(0, 60)]),
            &path,
            &settings(128, 72, 30.0),
            |_| {},
            &Arc::new(AtomicBool::new(true)),
        );
        assert!(matches!(done, Ok(false)), "{done:?}");
        assert!(!path.exists(), "the file of a cancelled export is deleted");
    }

    #[test]
    fn an_export_that_fails_leaves_no_file() {
        let path = file("failed");
        let missing = VideoPiece::new(test_fixture("no_such_file.mp4"), 0.0, 1.0);
        let done = write(
            &VideoSource::from_pieces([ramp(0, 10), missing]),
            &path,
            &settings(128, 72, 30.0),
            |_| {},
            &Arc::default(),
        );
        assert!(done.is_err(), "a list with a missing file is written: {done:?}");
        assert!(!path.exists(), "the file of a failed export is deleted");

        let none = write(
            &VideoSource::from_pieces([]),
            &path,
            &settings(128, 72, 30.0),
            |_| {},
            &Arc::default(),
        );
        assert!(none.is_err(), "a list with no pieces is written");
    }
}
