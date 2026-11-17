use std::{env::temp_dir, fs::write};

const CHECK_1: &str = r"
    336   48 - #202d36
    388   48 - #000000
    440   48 - #000000
    336   52 - #202d36
    92   56 - #000000
    484   56 - #364b5a
    236   60 - #000001
    352   60 - #507086
    484   60 - #364b5a
    312   64 - #597c95
    412   64 - #000000
    484   64 - #364b5a
    148   68 - #000000
    200   68 - #000000
    276   68 - #000001
    484   68 - #364b5a
    228  180 - #e94c3d
    340  200 - #e94c3d
    456  216 - #e94c3d
    140  256 - #e94c3d
    420  328 - #445e71
    180  340 - #597c95
    200  340 - #507086
    328  340 - #597c95
    216  344 - #597c95
    272  344 - #597c95
    372  344 - #425c6e
    400  348 - #000000
    592  404 - #597c95
    4  592 - #597c95
    320  592 - #597c95
    592  592 - #597c95
";

const CHECK_2: &str = r"
    336   48 - #202d36
    388   48 - #000000
    440   48 - #000000
    336   52 - #202d36
    92   56 - #000000
    484   56 - #364b5a
    236   60 - #000001
    352   60 - #507086
    484   60 - #364b5a
    312   64 - #597c95
    412   64 - #000000
    484   64 - #364b5a
    148   68 - #000000
    200   68 - #000000
    276   68 - #000001
    484   68 - #364b5a
    228  180 - #2fcc73
    340  200 - #2fcc73
    456  216 - #2fcc73
    140  256 - #2fcc73
    420  328 - #445e71
    180  340 - #597c95
    200  340 - #507086
    328  340 - #597c95
    216  344 - #597c95
    272  344 - #597c95
    372  344 - #425c6e
    400  348 - #000000
    592  404 - #597c95
    4  592 - #597c95
    320  592 - #597c95
    592  592 - #597c95
";

const CHECK_3: &str = r"
    336   48 - #202d36
    388   48 - #000000
    440   48 - #000000
    336   52 - #202d36
    92   56 - #000000
    484   56 - #364b5a
    236   60 - #000001
    352   60 - #507086
    484   60 - #364b5a
    312   64 - #597c95
    412   64 - #000000
    484   64 - #364b5a
    148   68 - #000000
    200   68 - #000000
    276   68 - #000001
    484   68 - #364b5a
    228  180 - #f1c310
    340  200 - #f1c310
    456  216 - #f1c310
    140  256 - #f1c310
    420  328 - #445e71
    180  340 - #597c95
    200  340 - #507086
    328  340 - #597c95
    216  344 - #597c95
    272  344 - #597c95
    372  344 - #425c6e
    400  348 - #000000
    592  404 - #597c95
    4  592 - #597c95
    320  592 - #597c95
    592  592 - #597c95
";

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

/// The 4 solid frames of the playback fixture, red, green, blue, yellow at
/// one per second, coded as AV1 with every frame a keyframe. No sound.
const VIDEO: &[u8] = include_bytes!("av1.mp4");

/// One second on the 60 fps stepped timeline.
const SECOND: u32 = 60;

/// Plays an AV1 file. Proves the archive decodes AV1 in software through
/// dav1d on a machine whose hardware has no AV1 decoder, the frames reach
/// the screen in their colors and in time. A Mac with a hardware AV1
/// decoder still decodes through dav1d here, see the roadmap.
#[view]
struct VideoAv1 {
    #[init]
    title: Label,
    video: VideoView,
    state: Label,
}

impl Setup for VideoAv1 {
    fn setup(self: Weak<Self>) {
        let path = temp_dir().join("hilen-video-av1.mp4");
        write(&path, VIDEO).expect("the fixture video is writable to the temp dir");
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("an AV1 file, decoded by dav1d");
        self.video.set_frame((140, 100, 320, 200));
        self.video.set_mode(ImageMode::Fill).set_source(path.to_string_lossy());
        self.state.set_frame((20, 320, 560, 40));
    }
}

impl ViewTest for VideoAv1 {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);
        step_frames(1);
        ensure!(
            from_main(move || view.video.is_loaded()),
            "the AV1 fixture must open"
        );

        let decoder = from_main(move || {
            let stats = view.video.stats();
            view.state.set_text(format!("decoder: {}", stats.decoder));
            stats.decoder
        });
        ensure!(decoder == "libdav1d", "AV1 decodes through dav1d, got {decoder}");
        step_frames(1);
        check_colors(CHECK_1)?;

        from_main(move || {
            view.video.play();
        });
        // A frame shows half a frame interval early, so green lands at 0.5 s.
        step_frames(SECOND * 3 / 4);
        check_colors(CHECK_2)?;
        step_frames(SECOND * 2);
        check_colors(CHECK_3)?;

        from_main(move || {
            view.video.pause();
        });
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
