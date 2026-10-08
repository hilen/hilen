//! Pictures of a video with no view and no clock, for a strip of thumbnails
//! or the picture of a clip in a list. One open decoder serves many places
//! of one file, exact to the frame. Opening also tells the length, the size
//! and the frame rate of the file before anything plays.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::Builder,
};

use anyhow::{Result, anyhow};
use ffmpeg_next::{Error, format::Pixel, frame, software::scaling};
use log::error;
use parking_lot::Mutex;

use crate::{
    deps::{
        hreads::on_main,
        refs::{Weak, manage::DataManager},
    },
    gm::{LossyConvert, flat::Size},
    video::{
        VideoSource,
        decoder::{
            Matrix,
            reader::{PictureColor, Reader},
        },
        source::Interrupt,
    },
    window::image::Image,
};

/// Seconds ahead of the last picture that are decoded through with no seek.
/// A seek goes back to a keyframe, which for a place this near is most often
/// the one decoding already passed.
const FORWARD: f64 = 2.0;

/// What a video file is, read before anything plays.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoInfo {
    /// Seconds.
    pub duration:   f64,
    /// Pixels.
    pub width:      u32,
    pub height:     u32,
    /// Frames per second.
    pub frame_rate: f64,
    /// The file has a sound track.
    pub has_sound:  bool,
}

/// One picture of a video as pixels, 4 bytes each in the order red, green,
/// blue, alpha, the rows from the top down with no padding.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoPicture {
    /// The timestamp of the frame the picture is, in seconds.
    pub seconds: f64,
    pub width:   u32,
    pub height:  u32,
    pub rgba:    Vec<u8>,
}

impl VideoPicture {
    /// The picture as an image a view can show. Main thread only. The image
    /// stays in memory under `name`, so the same name gives the same image.
    pub fn into_image(self, name: impl Into<String>) -> Weak<Image> {
        Image::from_raw_data(self.rgba, name, Size::new(self.width, self.height), 4)
    }
}

/// The frame handed out last, in system memory.
struct Last {
    seconds: f64,
    frame:   frame::Video,
    color:   PictureColor,
}

/// The open decoder of one video file. Every call reads the file and blocks,
/// so it lives on a thread of the app, never on the main thread.
/// `VideoFrames::load` is the same work with the thread included.
pub struct VideoFrames {
    reader: Reader,
    info:   VideoInfo,
    rgba:   ToRgba,
    last:   Option<Last>,
}

impl VideoFrames {
    /// Opens a file path or a url. Blocks while it reads the start of the
    /// file.
    pub fn open(source: impl Into<VideoSource>) -> Result<Self> {
        let source = source.into();
        let stop = Arc::new(AtomicBool::new(false));
        Self::open_with(&source, &Interrupt::new(&stop))
    }

    fn open_with(source: &VideoSource, reads: &Interrupt) -> Result<Self> {
        let reader = Reader::open(source, reads)
            .map_err(|err| anyhow!("video {} does not open: {err}", source.location()))?;
        let file = reader.info();
        let info = VideoInfo {
            duration:   file.duration,
            width:      file.width,
            height:     file.height,
            frame_rate: file.frame_rate,
            has_sound:  file.has_sound,
        };
        Ok(Self {
            reader,
            info,
            rgba: ToRgba::default(),
            last: None,
        })
    }

    pub fn info(&self) -> &VideoInfo {
        &self.info
    }

    /// The picture at these seconds, the frame a `VideoView` shows after a
    /// seek to the same place. It is scaled down to fit into `max` with its
    /// shape kept, never up. A zero in `max` puts no limit on that side. A
    /// place past the end gives the last frame.
    pub fn picture(&mut self, seconds: f64, max: impl Into<Size<u32>>) -> Result<VideoPicture> {
        self.step_to(seconds)?;
        let last = self.last.as_ref().ok_or_else(|| anyhow!("the video has no picture"))?;
        let size = fit(Size::new(last.frame.width(), last.frame.height()), max.into());
        let rgba = self.rgba.run(&last.frame, last.color, size)?;
        Ok(VideoPicture {
            seconds: last.seconds,
            width: size.width,
            height: size.height,
            rgba,
        })
    }

    /// The pictures at many places of the file in one pass. They are decoded
    /// in the order of time, whatever order `times` has, and `each` gets
    /// every picture with its index in `times`.
    pub fn pictures(
        &mut self,
        times: &[f64],
        max: impl Into<Size<u32>>,
        mut each: impl FnMut(usize, VideoPicture),
    ) -> Result<()> {
        let max = max.into();
        for index in in_time_order(times) {
            each(index, self.picture(times[index], max)?);
        }
        Ok(())
    }

    /// Makes `last` the frame at these seconds.
    fn step_to(&mut self, seconds: f64) -> Result<()> {
        let half = self.reader.half_frame();
        let ahead = self.last.as_ref().map(|last| seconds - last.seconds);
        match ahead {
            // Still the frame in hand.
            Some(ahead) if ahead >= -half && ahead < half => return Ok(()),
            Some(ahead) if ahead >= half && ahead <= FORWARD => self.reader.skip_to(seconds),
            _ => {
                self.last = None;
                self.reader.seek(seconds)?;
            }
        }
        let Some(picture) = self.reader.next()? else {
            // Nothing came after the frame in hand, it is the last one.
            return Ok(());
        };
        let (seconds, color) = (picture.seconds, picture.color());
        self.last = Some(Last {
            seconds,
            frame: picture.into_software()?,
            color,
        });
        Ok(())
    }
}

/// The indexes of `times` from the earliest to the latest.
fn in_time_order(times: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..times.len()).collect();
    order.sort_by(|a, b| times[*a].total_cmp(&times[*b]));
    order
}

/// The size of a picture that fits into `max` with its shape kept. A zero in
/// `max` is no limit on that side. Never bigger than the picture.
fn fit(picture: Size<u32>, max: Size<u32>) -> Size<u32> {
    let side = |limit: u32, full: u32| {
        if limit == 0 || full == 0 {
            1.0
        } else {
            f64::from(limit) / f64::from(full)
        }
    };
    let scale = side(max.width, picture.width).min(side(max.height, picture.height)).min(1.0);
    let scaled = |full: u32| -> u32 { (f64::from(full) * scale).round().max(1.0).lossy_convert() };
    Size::new(scaled(picture.width), scaled(picture.height))
}

/// What a scaler was built for.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Scaling {
    format: Pixel,
    from:   Size<u32>,
    to:     Size<u32>,
}

/// Decoder frames to RGBA at another size, in system memory. swscale only
/// scales, into planar YUV with a sample per pixel, and the colors are
/// computed here with the numbers of `nv12.wgsl`, so a picture has the
/// colors the view shows. One scaler, rebuilt when the frames or the size
/// change. 10 bit content is cut to 8 bits and HDR is not tone mapped.
#[derive(Default)]
struct ToRgba {
    built:  Option<Scaling>,
    scaler: Option<scaling::Context>,
}

impl ToRgba {
    fn run(&mut self, frame: &frame::Video, color: PictureColor, to: Size<u32>) -> Result<Vec<u8>, Error> {
        let scaling = Scaling {
            format: frame.format(),
            from: Size::new(frame.width(), frame.height()),
            to,
        };
        if self.built != Some(scaling) {
            self.scaler = Some(scaling::Context::get(
                scaling.format,
                scaling.from.width,
                scaling.from.height,
                Pixel::YUV444P,
                to.width,
                to.height,
                scaling::Flags::BILINEAR | scaling::Flags::ACCURATE_RND,
            )?);
            self.built = Some(scaling);
        }
        let scaler = self.scaler.as_mut().expect("the scaler was just made");
        let mut yuv = frame::Video::empty();
        scaler.run(frame, &mut yuv)?;

        let side = |pixels: u32| usize::try_from(pixels).expect("a picture side fits usize");
        let (width, height) = (side(to.width), side(to.height));
        let mut rgba = Vec::with_capacity(width * height * 4);
        let rows = |plane: usize| yuv.data(plane).chunks(yuv.stride(plane)).take(height);
        for ((y, u), v) in rows(0).zip(rows(1)).zip(rows(2)) {
            for ((y, u), v) in y.iter().zip(u).zip(v).take(width) {
                rgba.extend_from_slice(&rgb(*y, *u, *v, color));
                rgba.push(u8::MAX);
            }
        }
        Ok(rgba)
    }
}

/// One pixel from its 8 bit samples, the math of `nv12.wgsl` for SDR.
fn rgb(y: u8, u: u8, v: u8, color: PictureColor) -> [u8; 3] {
    let mut y = f32::from(y) / 255.0;
    let mut u = f32::from(u) / 255.0 - 0.5;
    let mut v = f32::from(v) / 255.0 - 0.5;
    if !color.full_range {
        y = (y - 16.0 / 255.0) * (255.0 / 219.0);
        u *= 255.0 / 224.0;
        v *= 255.0 / 224.0;
    }
    let rgb = match color.matrix {
        Matrix::Bt709 => [y + 1.5748 * v, y - 0.187_324 * u - 0.468_124 * v, y + 1.8556 * u],
        Matrix::Bt601 => [y + 1.402 * v, y - 0.344_136 * u - 0.714_136 * v, y + 1.772 * u],
        Matrix::Bt2020 => [y + 1.4746 * v, y - 0.164_553 * u - 0.571_353 * v, y + 1.8814 * u],
    };
    rgb.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round().lossy_convert())
}

/// What `VideoFrames::load` reports, on the main thread and in this order:
/// `Info`, a `Frame` per place asked for in the order of time, `Finished`.
/// `Failed` ends it at any point.
pub enum VideoFramesEvent {
    /// The file opened.
    Info(VideoInfo),
    Frame {
        /// The index of the place in the `times` given to `load`.
        index:   usize,
        /// The timestamp of the frame, in seconds.
        seconds: f64,
        image:   Weak<Image>,
    },
    Finished,
    /// The file did not open or decode, with the reason.
    Failed(String),
}

/// A `VideoFrames::load` that runs. Dropping it changes nothing, the load
/// goes on.
pub struct VideoFramesLoad {
    cancelled: Arc<AtomicBool>,
}

impl VideoFramesLoad {
    /// Ends the load. No event comes after this call, also not `Finished`.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl VideoFrames {
    /// Reads the pictures of a file at `times` on a thread of its own and
    /// hands them over on the main thread as images, see `VideoFramesEvent`.
    /// Call it with no `times` to get only the `Info`. The pictures fit into
    /// `max` like those of `picture`. An image stays in memory for good
    /// under the name of its source, its place and `max`, so asking again
    /// for the same picture decodes nothing.
    pub fn load(
        source: impl Into<VideoSource>,
        times: impl IntoIterator<Item = f64>,
        max: impl Into<Size<u32>>,
        each: impl FnMut(VideoFramesEvent) + Send + 'static,
    ) -> VideoFramesLoad {
        let source = source.into();
        let times: Vec<f64> = times.into_iter().collect();
        let max = max.into();
        let cancelled = Arc::new(AtomicBool::new(false));

        let report = Reporter {
            each:      Arc::new(Mutex::new(each)),
            cancelled: Arc::clone(&cancelled),
        };
        let location = source.location().to_string();
        let failed = report.clone();
        let spawned = Builder::new().name("hilen-video-frames".into()).spawn(move || {
            // A cancel ends a read with an error, that is not a failure.
            if let Err(err) = load(&source, &times, max, &report)
                && !report.cancelled()
            {
                error!("video {}: {err:#}", source.location());
                report.send(VideoFramesEvent::Failed(format!("{err:#}")));
            }
        });
        if let Err(err) = spawned {
            error!("video {location}: no thread to read the pictures, {err}");
            failed.send(VideoFramesEvent::Failed(err.to_string()));
        }
        VideoFramesLoad { cancelled }
    }
}

/// Hands the events of a load to the app, on the main thread.
struct Reporter<F> {
    each:      Arc<Mutex<F>>,
    cancelled: Arc<AtomicBool>,
}

impl<F> Clone for Reporter<F> {
    fn clone(&self) -> Self {
        Self {
            each:      Arc::clone(&self.each),
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

impl<F: FnMut(VideoFramesEvent) + Send + 'static> Reporter<F> {
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    fn send(&self, event: VideoFramesEvent) {
        self.on_main(move || event);
    }

    /// `make` runs on the main thread, an image is made there.
    fn on_main(&self, make: impl FnOnce() -> VideoFramesEvent + Send + 'static) {
        let this = self.clone();
        on_main(move || {
            if !this.cancelled() {
                (this.each.lock())(make());
            }
        });
    }
}

fn load<F: FnMut(VideoFramesEvent) + Send + 'static>(
    source: &VideoSource,
    times: &[f64],
    max: Size<u32>,
    report: &Reporter<F>,
) -> Result<()> {
    let reads = Interrupt::new(&report.cancelled);
    let mut frames = VideoFrames::open_with(source, &reads)?;
    report.send(VideoFramesEvent::Info(frames.info().clone()));

    for index in in_time_order(times) {
        if report.cancelled() {
            return Ok(());
        }
        let seconds = times[index];
        let name = image_name(source, seconds, max);
        if let Some(image) = Image::get_existing(&name) {
            report.send(VideoFramesEvent::Frame {
                index,
                seconds,
                image,
            });
            continue;
        }
        let picture = frames.picture(seconds, max)?;
        report.on_main(move || VideoFramesEvent::Frame {
            index,
            seconds: picture.seconds,
            image: picture.into_image(name),
        });
    }
    report.send(VideoFramesEvent::Finished);
    Ok(())
}

/// The name an image of a load is kept under, to the millisecond.
fn image_name(source: &VideoSource, seconds: f64, max: Size<u32>) -> String {
    let millis: i64 = (seconds * 1000.0).round().lossy_convert();
    format!(
        "video-frame {} {millis} {}x{}",
        source.location(),
        max.width,
        max.height
    )
}

#[cfg(test)]
mod test {
    use crate::{
        gm::{LossyConvert, flat::Size},
        video::{
            frames::{VideoFrames, VideoPicture, fit},
            test_fixture,
        },
    };

    /// The gray of frame `n` of the ramp fixture is luma `16 + 3 n` in
    /// limited range, so `3.49 n` as an RGB value.
    fn ramp_index(picture: &VideoPicture) -> i32 {
        let middle = usize::try_from(picture.height / 2 * picture.width + picture.width / 2)
            .expect("a pixel index fits usize")
            * 4;
        let pixel = &picture.rgba[middle..middle + 4];
        assert_eq!(pixel[3], 255, "the picture is opaque");
        assert!(
            pixel[0].abs_diff(pixel[1]) <= 1 && pixel[1].abs_diff(pixel[2]) <= 1,
            "the fixture is gray, got {pixel:?}"
        );
        (f64::from(pixel[1]) * 219.0 / 255.0 / 3.0).round().lossy_convert()
    }

    #[test]
    fn a_file_tells_what_it_is() {
        let frames = VideoFrames::open(test_fixture("ramp.mp4")).expect("the fixture opens");
        let info = frames.info();
        assert_eq!((info.width, info.height), (128, 72));
        assert!((info.duration - 2.0).abs() < 0.01, "{}", info.duration);
        assert!((info.frame_rate - 30.0).abs() < 0.01, "{}", info.frame_rate);
        assert!(info.has_sound);

        let silent = VideoFrames::open(test_fixture("colors.mp4")).expect("the fixture opens");
        assert!(!silent.info().has_sound);
        assert!((silent.info().duration - 4.0).abs() < 0.01);

        assert!(VideoFrames::open(test_fixture("no_such_file.mp4")).is_err());
    }

    /// Every frame of the fixture, asked for one by one in an order that
    /// makes the decoder go back, ahead by a little and ahead by a lot.
    #[test]
    fn the_picture_at_a_place_is_the_exact_frame() {
        let mut frames = VideoFrames::open(test_fixture("ramp.mp4")).expect("the fixture opens");
        for frame in [40, 41, 41, 45, 7, 59, 0, 30, 29] {
            let seconds = f64::from(frame) / 30.0;
            let picture = frames.picture(seconds, (0, 0)).expect("the fixture decodes");
            assert_eq!((picture.width, picture.height), (128, 72));
            assert_eq!(picture.rgba.len(), 128 * 72 * 4);
            assert!((picture.seconds - seconds).abs() < 0.001);
            assert_eq!(ramp_index(&picture), frame, "the frame at {seconds} s");
        }
        // Past the end the last frame comes, as often as it is asked for.
        for _ in 0..2 {
            let picture = frames.picture(9.0, (0, 0)).expect("the fixture decodes");
            assert_eq!(ramp_index(&picture), 59);
        }
    }

    /// A batch is decoded in the order of time and every picture comes with
    /// the index of its place in the list.
    #[test]
    fn a_batch_is_served_in_time_order() {
        let mut frames = VideoFrames::open(test_fixture("ramp.mp4")).expect("the fixture opens");
        let wanted = [50, 10, 30, 20, 0, 59];
        let times: Vec<f64> = wanted.iter().map(|frame| f64::from(*frame) / 30.0).collect();
        let mut seen = Vec::new();
        frames
            .pictures(&times, (64, 0), |index, picture| {
                assert_eq!((picture.width, picture.height), (64, 36));
                seen.push((index, ramp_index(&picture)));
            })
            .expect("the fixture decodes");
        assert_eq!(seen, [(4, 0), (1, 10), (3, 20), (2, 30), (0, 50), (5, 59)]);
    }

    /// The 4 frames of the color fixture, red, green, blue and yellow, each
    /// a second long. A wrong matrix or range moves these colors.
    #[test]
    fn the_colors_of_a_picture_are_those_of_the_video() {
        let mut frames = VideoFrames::open(test_fixture("colors.mp4")).expect("the fixture opens");
        let colors = [
            (0.0, [0xe9, 0x4c, 0x3d]),
            (1.2, [0x2f, 0xcc, 0x73]),
            (2.0, [0x34, 0x97, 0xdb]),
            (3.4, [0xf1, 0xc3, 0x10]),
        ];
        for (seconds, color) in colors {
            let picture = frames.picture(seconds, (16, 16)).expect("the fixture decodes");
            assert_eq!((picture.width, picture.height), (16, 16));
            let pixel = &picture.rgba[(8 * 16 + 8) * 4..][..3];
            let near = pixel.iter().zip(color).all(|(have, want)| have.abs_diff(want) <= 3);
            assert!(near, "at {seconds} s the color is {pixel:?}, not {color:?}");
        }
    }

    #[test]
    fn a_picture_fits_into_the_size_asked_for() {
        let full = Size::new(1920, 1080);
        assert_eq!(fit(full, Size::new(0, 0)), full);
        assert_eq!(fit(full, Size::new(160, 0)), Size::new(160, 90));
        assert_eq!(fit(full, Size::new(0, 54)), Size::new(96, 54));
        assert_eq!(fit(full, Size::new(160, 54)), Size::new(96, 54));
        assert_eq!(fit(full, Size::new(4000, 4000)), full);
        assert_eq!(fit(Size::new(1000, 2), Size::new(10, 10)), Size::new(10, 1));
    }
}
