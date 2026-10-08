use std::{
    env::temp_dir,
    fs::write,
    path::PathBuf,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Label, Setup, TextAlignment, VideoView, ViewFrame, ViewTest, view},
    ui_test::check_colors,
    video::{VideoExport, VideoExportEvent, VideoExportSettings, VideoPiece, VideoState},
};

/// 60 frames at 30 per second with a sound track. Frame `n` is a flat gray
/// that gets lighter with `n`, luma `16 + 3 n`.
const RAMP: &[u8] = include_bytes!("ramp.mp4");
/// Four solid frames at one per second, red, green, blue, yellow, square.
const COLORS: &[u8] = include_bytes!("colors.mp4");

/// The middle of the video on the canvas, and a place in the black at the
/// side of the square piece.
const MIDDLE: (u32, u32) = (180, 210);
const SIDE: (u32, u32) = (40, 210);

const WRITTEN: &str = r"
     40   64 - #597c95
    112   64 - #1b252c
    232   64 - #141b21
    172   68 - #151d23
    204   68 - #273641
    304   68 - #456073
    320   92 - #597c95
    412   92 - #597c95
     52   96 - #415a6c
    108   96 - #25343f
    168   96 - #19232b
    212   96 - #3c5364
    264   96 - #597c95
    488   96 - #395060
     68  100 - #2f424f
    356  100 - #000000
     20  188 - #aeafb0
    140  204 - #aeafb0
    252  204 - #aeafb0
     80  296 - #aeafb0
    592  308 - #597c95
    312  332 - #000000
     40  336 - #597c95
     92  336 - #000001
    132  336 - #425c6e
    200  336 - #000000
    232  336 - #3c5465
    360  336 - #090c0f
    264  340 - #263540
      4  592 - #597c95
    328  592 - #597c95
    592  592 - #597c95
";
const SECOND_PIECE: &str = r"
     40   64 - #597c95
    112   64 - #1b252c
    232   64 - #141b21
    172   68 - #151d23
    276   68 - #1d2830
    304   68 - #456073
     88   88 - #24323c
    144   88 - #24323c
    320   92 - #597c95
     52   96 - #415a6c
    212   96 - #3c5364
    488   96 - #395060
    380  100 - #000000
    272  148 - #01020c
    124  168 - #3497db
    336  192 - #010001
     20  204 - #010001
    192  216 - #3497db
    104  248 - #3497db
    268  252 - #4793c8
     20  280 - #010001
    136  332 - #000001
    316  332 - #0c1114
     72  336 - #415a6d
    196  336 - #597c95
    256  336 - #597c95
    388  336 - #2e414e
    444  340 - #010102
    428  520 - #597c95
      4  588 - #597c95
    264  592 - #597c95
    592  592 - #597c95
";
const CANCELLED: &str = r"
     40   64 - #597c95
    100   64 - #4a677c
    232   64 - #141b21
    276   68 - #1d2830
    144   88 - #24323c
    212   96 - #3c5364
    488   96 - #395060
     68  100 - #2f424f
    324  100 - #000000
    380  100 - #000000
     88  152 - #000414
    260  164 - #3497db
    336  192 - #010001
     20  204 - #010001
    192  216 - #3497db
    592  240 - #597c95
    104  248 - #3497db
    268  260 - #4793c8
     20  280 - #010001
    120  332 - #597c95
    316  332 - #0c1114
     72  336 - #415a6d
    196  336 - #597c95
    240  336 - #456073
    444  340 - #010102
     44  368 - #151d23
    272  368 - #11181d
    368  368 - #597c95
    148  372 - #384e5e
      4  592 - #597c95
    320  592 - #597c95
    592  592 - #597c95
";

fn temp(name: &str) -> PathBuf {
    temp_dir().join(format!("hilen-video-export-test-{name}.mp4"))
}

fn ramp(first: u32, end: u32) -> VideoPiece {
    VideoPiece::new(
        temp("ramp").to_string_lossy(),
        f64::from(first) / 30.0,
        f64::from(end) / 30.0,
    )
}

/// Polls the main thread until `done` holds.
fn wait_until(what: &str, done: impl Fn() -> bool + Send + Copy + 'static) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !from_main(done) {
        if Instant::now() > deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Writes a list of pieces into an mp4 file and plays the file. Proves that
/// an export reports its progress up to 1 and then that it finished, all on
/// the main thread, that the file plays with the frames of the list at
/// their places and black around a piece of another shape, and that a
/// cancelled export reports that and leaves no file.
#[view]
struct VideoExportToFile {
    events:    Vec<VideoExportEvent>,
    export:    Option<VideoExport>,
    cancelled: Vec<VideoExportEvent>,

    #[init]
    title:  Label,
    list:   Label,
    video:  VideoView,
    status: Label,
    second: Label,
}

impl VideoExportToFile {
    fn note(label: Weak<Label>, y: u32, text: &str) {
        label.set_frame((20, y, 560, 30));
        label.set_text_size(18).set_alignment(TextAlignment::Left).set_text(text);
    }

    fn took(mut self: Weak<Self>, event: VideoExportEvent) {
        match &event {
            VideoExportEvent::Progress(_) => self.status.set_text("the export runs"),
            VideoExportEvent::Finished => {
                // The file plays like any other video.
                self.video.set_source(temp("written").to_string_lossy());
                self.status.set_text("the export finished, the view plays the file")
            }
            VideoExportEvent::Cancelled => self.status.set_text("the export was cancelled"),
            VideoExportEvent::Failed(reason) => self.status.set_text(format!("the export failed: {reason}")),
        };
        self.events.push(event);
    }
}

impl Setup for VideoExportToFile {
    fn setup(mut self: Weak<Self>) {
        write(temp("ramp"), RAMP).expect("the fixture video is writable to the temp dir");
        write(temp("colors"), COLORS).expect("the fixture video is writable to the temp dir");

        Self::note(self.title, 50, "a list of pieces written into an mp4 file");
        Self::note(
            self.list,
            80,
            "ramp 50 to 55, then blue for 0.5 s, 128 by 72 at 30 a second",
        );
        self.video.set_frame((20, 120, 320, 180));
        self.video.set_mode(ImageMode::Fill);
        Self::note(self.status, 320, "the export starts");
        Self::note(self.second, 350, "");

        let blue = VideoPiece::new(temp("colors").to_string_lossy(), 2.0, 2.5);
        let settings = VideoExportSettings {
            video_bit_rate: 3_000_000,
            ..VideoExportSettings::new(128, 72, 30.0)
        };
        self.export = Some(VideoExport::start(
            [ramp(50, 56), blue],
            temp("written"),
            settings,
            move |event| self.took(event),
        ));
    }
}

/// The color in the video at a place of the canvas.
fn color_at(place: (u32, u32)) -> Result<(u8, u8, u8)> {
    let pixel = AppRunner::take_screenshot()?.get_pixel(place);
    Ok((pixel.r, pixel.g, pixel.b))
}

impl ViewTest for VideoExportToFile {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the export to end", move || {
            view.events.iter().any(|event| !matches!(event, VideoExportEvent::Progress(_)))
        })?;
        let events = from_main(move || view.events.clone());
        let Some((last, progress)) = events.split_last() else {
            bail!("the export reported nothing");
        };
        ensure!(
            *last == VideoExportEvent::Finished,
            "the export ended with {last:?}"
        );
        let parts: Vec<f32> = progress
            .iter()
            .filter_map(|event| match event {
                VideoExportEvent::Progress(part) => Some(*part),
                _ => None,
            })
            .collect();
        ensure!(
            parts.len() == progress.len() && parts.is_sorted(),
            "only progress that never goes back comes before the end, got {progress:?}"
        );
        ensure!(
            parts.last().is_some_and(|part| (part - 1.0).abs() < f32::EPSILON),
            "the progress ends at 1, got {parts:?}"
        );
        let handle = from_main(move || view.export.as_ref().map(VideoExport::progress));
        ensure!(
            handle.is_some_and(|part| (part - 1.0).abs() < f32::EPSILON),
            "the progress of the handle ends at 1, got {handle:?}"
        );

        Self::the_file_plays(view)
    }
}

impl VideoExportToFile {
    /// The second half of the test: the written file in a view, then an
    /// export that is cancelled.
    fn the_file_plays(mut view: Weak<Self>) -> Result<()> {
        // The file plays. Its first frame is the first frame of the list.
        wait_until("the first frame of the file", move || {
            view.video.state() == VideoState::Paused
        })?;
        let duration = from_main(move || view.video.duration());
        ensure!(
            (duration - 0.7).abs() < 0.05,
            "6 frames and half a second are 0.7 seconds, the file has {duration}"
        );
        // Ramp 50 is luma 166, a gray of 174 on screen. The encoder loses a
        // little, the frames of the ramp are 3.5 apart.
        let (r, g, b) = color_at(MIDDLE)?;
        ensure!(
            r.abs_diff(174) <= 5 && g.abs_diff(174) <= 5 && b.abs_diff(174) <= 5,
            "the first frame of the file is ({r}, {g}, {b}), not the gray of ramp 50"
        );
        check_colors(WRITTEN)?;

        // 0.4 seconds in is the second piece, the square blue picture with
        // black at its sides.
        from_main(move || {
            view.status
                .set_text("the file at 0.4 s, the blue piece with black at its sides");
            view.video.seek_to(0.4);
        });
        // The frame of the seek comes some frames later. Blue is the only
        // picture with more blue than red.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let (r, _, b) = color_at(MIDDLE)?;
            if b > r.saturating_add(60) {
                break;
            }
            ensure!(Instant::now() < deadline, "the frame at 0.4 s never showed");
            sleep(Duration::from_millis(10));
        }
        let (r, g, b) = color_at(MIDDLE)?;
        ensure!(
            r.abs_diff(0x34) <= 16 && g.abs_diff(0x97) <= 16 && b.abs_diff(0xdb) <= 16,
            "the second piece is ({r}, {g}, {b}) in the middle, not blue"
        );
        let (r, g, b) = color_at(SIDE)?;
        ensure!(
            r <= 16 && g <= 16 && b <= 16,
            "the side of the square piece is ({r}, {g}, {b}), not black"
        );
        check_colors(SECOND_PIECE)?;

        // A second export is cancelled right after its start.
        from_main(move || {
            view.second.set_text("a second export starts and is cancelled");
            let export = VideoExport::start(
                [ramp(0, 60)],
                temp("cancelled"),
                VideoExportSettings::new(128, 72, 30.0),
                move |event| {
                    if event == VideoExportEvent::Cancelled {
                        view.second.set_text("the second export was cancelled, no file is left");
                    }
                    view.cancelled.push(event);
                },
            );
            export.cancel();
        });
        wait_until("the cancel", move || {
            view.cancelled
                .iter()
                .any(|event| !matches!(event, VideoExportEvent::Progress(_)))
        })?;
        let last = from_main(move || view.cancelled.last().cloned());
        ensure!(
            last == Some(VideoExportEvent::Cancelled),
            "the cancelled export ended with {last:?}"
        );
        ensure!(
            !temp("cancelled").exists(),
            "the file of a cancelled export is deleted"
        );
        check_colors(CANCELLED)?;
        Ok(())
    }
}
