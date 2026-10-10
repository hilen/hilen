use std::{env::temp_dir, fs::write};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{BLACK, ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, step_frames},
};

/// Four solid frames at one per second, red, green, blue, yellow.
const VIDEO: &[u8] = include_bytes!("colors.mp4");

const OVER_RED: &str = r"
    536   60 - #e94c3d
     60   64 - #e94c3d
    256  124 - #ffffff
    288  124 - #ffffff
    316  124 - #ffffff
    304  144 - #e94c3d
    340  144 - #fef4f3
    268  156 - #f2938a
    288  156 - #f2938a
     60  252 - #e94c3d
    232  320 - #000000
    276  320 - #000000
    304  324 - #000000
    308  324 - #1f1f1f
    344  324 - #822a22
    256  332 - #000000
    244  336 - #e94c3d
    324  336 - #9c9c9c
    360  340 - #fefefe
    380  340 - #000000
    228  344 - #e94c3d
    284  344 - #6b231c
    296  344 - #fefefe
    320  344 - #bdbdbd
    344  344 - #822a22
    324  348 - #9c9c9c
    344  348 - #822a22
    320  352 - #bdbdbd
    268  356 - #656565
      4  592 - #597c95
    348  592 - #597c95
    592  592 - #597c95
";

const OVER_GREEN: &str = r"
    536   60 - #2fcc73
     60   64 - #2fcc73
    256  124 - #ffffff
    288  124 - #ffffff
    316  124 - #ffffff
    304  144 - #2fcc73
    340  144 - #f2fcf6
    268  156 - #81e0aa
    288  156 - #81e0aa
     60  252 - #2fcc73
    232  320 - #000000
    276  320 - #000000
    304  324 - #000000
    308  324 - #1f1f1f
    344  324 - #1a7240
    256  332 - #000000
    244  336 - #2fcc73
    324  336 - #9c9c9c
    360  340 - #fefefe
    380  340 - #000000
    228  344 - #2fcc73
    284  344 - #165e35
    296  344 - #fefefe
    320  344 - #bdbdbd
    344  344 - #1a7240
    324  348 - #9c9c9c
    344  348 - #1a7240
    320  352 - #bdbdbd
    268  356 - #656565
      4  592 - #597c95
    348  592 - #597c95
    592  592 - #597c95
";

/// One second on the 60 fps stepped timeline.
const SECOND: u32 = 60;

/// A video whose picture fills its view, with 2 labels that an app put over
/// it after the video: a plain one, a title, and one with an outline, a
/// subtitle line. Proves that a later sibling of a `VideoView` draws over
/// its picture, before play and while it plays. The picture is drawn by a
/// child of the video view, and a child used to be nearer than a later
/// sibling of its parent, so both labels were hidden wherever the picture
/// was.
#[view]
struct VideoLabelOver {
    #[init]
    video:    VideoView,
    title:    Label,
    subtitle: Label,
}

impl Setup for VideoLabelOver {
    fn setup(self: Weak<Self>) {
        let path = temp_dir().join("hilen-video-label-over.mp4");
        write(&path, VIDEO).expect("the fixture video is writable to the temp dir");
        self.video.set_frame((60, 60, 480, 360));
        self.video.set_mode(ImageMode::Fill).set_source(path.to_string_lossy());

        self.title.set_text("Title").set_text_size(48).set_text_color(WHITE);
        self.title.set_frame((100, 100, 400, 80));

        self.subtitle
            .set_text("Subtitle")
            .set_text_size(48)
            .set_text_color(WHITE)
            .set_text_outline(BLACK, 3);
        self.subtitle.set_frame((100, 300, 400, 80));
    }
}

impl ViewTest for VideoLabelOver {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);

        step_frames(1);
        ensure!(from_main(move || view.video.is_loaded()), "the fixture must open");
        check_colors(OVER_RED)?;

        from_main(move || {
            view.video.play();
        });
        // A frame shows half a frame interval early, so green lands at 0.5 s.
        step_frames(SECOND * 3 / 4);
        check_colors(OVER_GREEN)?;

        from_main(move || {
            view.video.pause();
        });
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
