use std::{
    env::temp_dir,
    fs::write,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Image, ImageView, Label, Setup, TextAlignment, ViewFrame, ViewTest, view},
    ui_test::check_colors,
    video::{VideoFrames, VideoFramesEvent, VideoInfo},
};

/// Four solid frames at one per second, red, green, blue, yellow.
const COLORS: &[u8] = include_bytes!("colors.mp4");
/// 60 frames at 30 per second, frame `n` is a flat gray that gets lighter
/// with `n`, with 1 keyframe per second.
const RAMP: &[u8] = include_bytes!("ramp.mp4");

/// The frames of the ramp asked for, on purpose not in the order of time.
const RAMP_FRAMES: [u32; 4] = [40, 0, 59, 20];

/// Where the 4 pictures of a row start.
const COLUMNS: [u32; 4] = [20, 160, 300, 440];

const PICTURES: &str = r"
    460    4 - #597c95
    160   60 - #304350
     48   64 - #000000
    308   64 - #0f151a
    208   68 - #000000
    120  108 - #456073
    440  108 - #597c95
    564  132 - #f1c310
    352  144 - #3497db
    144  164 - #e94c3d
    264  176 - #2fcc73
    424  200 - #3497db
     20  204 - #e94c3d
    524  228 - #f1c310
    200  252 - #2fcc73
     52  296 - #415a6c
    112  296 - #506f85
    164  296 - #090c0f
    292  296 - #597c95
    372  320 - #8b8c8d
    476  320 - #cccdce
    564  320 - #cccdce
     20  364 - #010001
     96  372 - #010001
    284  376 - #454647
    204  388 - #454647
    424  388 - #8b8c8d
    132  440 - #11181d
     64  468 - #263540
    232  468 - #273641
    328  468 - #597c95
    592  592 - #597c95
";

/// Polls the main thread until `done` holds.
fn wait_until(what: &str, done: impl Fn() -> bool + Send + Copy + 'static) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !from_main(done) {
        if Instant::now() > deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// What one `VideoFrames::load` reported.
#[derive(Default)]
struct Load {
    info:     Option<VideoInfo>,
    /// The index and the timestamp of every picture, in the order they came.
    frames:   Vec<(usize, f64)>,
    images:   Vec<Weak<Image>>,
    finished: bool,
    failed:   Option<String>,
}

/// Shows pictures of 2 videos with no `VideoView`. Proves that a load
/// reports the length, the size and the frame rate first, that every picture
/// is the frame at its place with the colors of the video, that a batch
/// comes in the order of time with the index of each place, that the same
/// picture asked for again is the same image, and that a load that was
/// cancelled reports nothing.
#[view]
struct VideoThumbnails {
    colors:    Load,
    ramp:      Load,
    again:     Load,
    cancelled: Load,

    #[init]
    title:       Label,
    colors_note: Label,
    red:         ImageView,
    green:       ImageView,
    blue:        ImageView,
    yellow:      ImageView,
    ramp_note:   Label,
    frame_0:     ImageView,
    frame_20:    ImageView,
    frame_40:    ImageView,
    frame_59:    ImageView,
    status:      Label,
}

fn take(load: &mut Load, event: VideoFramesEvent) -> Option<(usize, Weak<Image>)> {
    match event {
        VideoFramesEvent::Info(info) => load.info = Some(info),
        VideoFramesEvent::Frame {
            index,
            seconds,
            image,
        } => {
            load.frames.push((index, seconds));
            load.images.push(image);
            return Some((index, image));
        }
        VideoFramesEvent::Finished => load.finished = true,
        VideoFramesEvent::Failed(reason) => load.failed = Some(reason),
    }
    None
}

impl VideoThumbnails {
    fn note(label: Weak<Label>, y: u32, text: &str) {
        label.set_frame((20, y, 560, 30));
        label.set_text_size(18).set_alignment(TextAlignment::Left).set_text(text);
    }

    fn load_colors(mut self: Weak<Self>, path: String) {
        let views = [self.red, self.green, self.blue, self.yellow];
        VideoFrames::load(path, [0.2, 1.2, 2.2, 3.2], (64, 64), move |event| {
            if let Some((index, image)) = take(&mut self.colors, event) {
                views[index].set_image(image);
            }
            self.describe();
        });
    }

    fn load_ramp(mut self: Weak<Self>, path: String) {
        // Frame 40 is asked for first and lands in the third view.
        let views = [self.frame_40, self.frame_0, self.frame_59, self.frame_20];
        let times = RAMP_FRAMES.map(|frame| f64::from(frame) / 30.0);
        VideoFrames::load(path, times, (0, 0), move |event| {
            if let Some((index, image)) = take(&mut self.ramp, event) {
                views[index].set_image(image);
            }
            self.describe();
        });
    }

    fn describe(self: Weak<Self>) {
        let info = |load: &Load| {
            load.info.as_ref().map_or_else(
                || "not open".to_string(),
                |info| {
                    format!(
                        "{:.0} s, {} by {}, {:.0} fps",
                        info.duration, info.width, info.height, info.frame_rate
                    )
                },
            )
        };
        self.status.set_text(format!(
            "colors: {}, {} pictures\nramp: {}, {} pictures",
            info(&self.colors),
            self.colors.frames.len(),
            info(&self.ramp),
            self.ramp.frames.len()
        ));
    }
}

impl Setup for VideoThumbnails {
    fn setup(self: Weak<Self>) {
        let colors = temp_dir().join("hilen-video-frames-colors.mp4");
        write(&colors, COLORS).expect("the fixture video is writable to the temp dir");
        let ramp = temp_dir().join("hilen-video-frames-ramp.mp4");
        write(&ramp, RAMP).expect("the fixture video is writable to the temp dir");

        Self::note(self.title, 50, "pictures of a video with no video view");
        Self::note(
            self.colors_note,
            90,
            "colors.mp4 at 0.2, 1.2, 2.2, 3.2 s: red, green, blue, yellow",
        );
        for (view, x) in [self.red, self.green, self.blue, self.yellow].into_iter().zip(COLUMNS) {
            view.set_frame((x, 130, 128, 128));
        }
        Self::note(
            self.ramp_note,
            280,
            "ramp.mp4 frames 0, 20, 40, 59: black to light gray",
        );
        let ramp_views = [self.frame_0, self.frame_20, self.frame_40, self.frame_59];
        for (view, x) in ramp_views.into_iter().zip(COLUMNS) {
            view.set_frame((x, 320, 128, 72));
        }
        self.status.set_frame((20, 420, 560, 70));
        self.status
            .set_text_size(18)
            .set_alignment(TextAlignment::Left)
            .set_multiline(true);
        self.describe();

        self.load_colors(colors.to_string_lossy().to_string());
        self.load_ramp(ramp.to_string_lossy().to_string());
    }
}

impl ViewTest for VideoThumbnails {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_until("both loads", move || {
            (view.colors.finished || view.colors.failed.is_some())
                && (view.ramp.finished || view.ramp.failed.is_some())
        })?;
        let failed = from_main(move || view.colors.failed.clone().or_else(|| view.ramp.failed.clone()));
        ensure!(failed.is_none(), "a load failed: {failed:?}");

        let info = from_main(move || view.ramp.info.clone());
        let Some(info) = info else {
            bail!("the ramp load reported no info");
        };
        ensure!(
            (info.width, info.height) == (128, 72)
                && (info.duration - 2.0).abs() < 0.01
                && (info.frame_rate - 30.0).abs() < 0.01
                && info.has_sound,
            "the ramp fixture is {info:?}"
        );

        // The batch came in the order of time, each with its own index.
        let frames = from_main(move || view.ramp.frames.clone());
        let order: Vec<usize> = frames.iter().map(|(index, _)| *index).collect();
        ensure!(order == [1, 3, 0, 2], "the ramp pictures came as {order:?}");
        for (index, seconds) in &frames {
            let wanted = f64::from(RAMP_FRAMES[*index]) / 30.0;
            ensure!(
                (seconds - wanted).abs() < 0.001,
                "picture {index} is the frame at {seconds}, not at {wanted}"
            );
        }
        let colors = from_main(move || view.colors.frames.len());
        ensure!(colors == 4, "the color load gave {colors} pictures");

        check_colors(PICTURES)?;

        // The same pictures asked for again are the same images.
        let path = temp_dir().join("hilen-video-frames-colors.mp4").to_string_lossy().to_string();
        let again = path.clone();
        from_main(move || {
            VideoFrames::load(again, [0.2, 1.2, 2.2, 3.2], (64, 64), move |event| {
                take(&mut view.again, event);
            });
        });
        wait_until("the second load", move || {
            view.again.finished || view.again.failed.is_some()
        })?;
        ensure!(
            from_main(move || view.again.images == view.colors.images),
            "a picture asked for again is not the image of the first time"
        );

        // A load that is cancelled reports nothing, also not what was on
        // its way to the main thread.
        from_main(move || {
            let load = VideoFrames::load(path, [0.7, 1.7], (32, 32), move |event| {
                take(&mut view.cancelled, event);
            });
            load.cancel();
        });
        sleep(Duration::from_millis(300));
        let reported = from_main(move || {
            let load = &view.cancelled;
            load.info.is_some() || !load.frames.is_empty() || load.finished || load.failed.is_some()
        });
        ensure!(!reported, "a cancelled load still reported");
        Ok(())
    }
}
