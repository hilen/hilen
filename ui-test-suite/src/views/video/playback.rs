use std::{env::temp_dir, fs::write, sync::mpsc::channel};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{ImageMode, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, step_frames},
};
use log::error;

/// Four solid frames at one per second, red, green, blue, yellow, h264 in an
/// mp4 with every frame a keyframe so a seek lands exactly. No sound, so the
/// picture follows the engine clock and stepped time drives it.
const VIDEO: &[u8] = include_bytes!("colors.mp4");

/// One second on the 60 fps stepped timeline.
const SECOND: u32 = 60;

const RED: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #e94c3d
    188  100 - #e94c3d
    308  100 - #e94c3d
    392  100 - #e94c3d
    148  144 - #e94c3d
    248  156 - #e94c3d
      4  168 - #597c95
    388  168 - #e94c3d
    188  188 - #e94c3d
    592  188 - #597c95
    120  200 - #e94c3d
    328  204 - #e94c3d
    264  232 - #e94c3d
    416  232 - #e94c3d
    200  256 - #e94c3d
    244  292 - #e94c3d
    100  296 - #e94c3d
    376  296 - #e94c3d
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const GREEN: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #2fcc73
    188  100 - #2fcc73
    308  100 - #2fcc73
    392  100 - #2fcc73
    148  144 - #2fcc73
    248  156 - #2fcc73
      4  168 - #597c95
    388  168 - #2fcc73
    188  188 - #2fcc73
    592  188 - #597c95
    120  200 - #2fcc73
    328  204 - #2fcc73
    264  232 - #2fcc73
    416  232 - #2fcc73
    200  256 - #2fcc73
    244  292 - #2fcc73
    100  296 - #2fcc73
    376  296 - #2fcc73
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const BLUE: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #3497db
    188  100 - #3497db
    308  100 - #3497db
    392  100 - #3497db
    148  144 - #3497db
    248  156 - #3497db
      4  168 - #597c95
    388  168 - #3497db
    188  188 - #3497db
    592  188 - #597c95
    120  200 - #3497db
    328  204 - #3497db
    264  232 - #3497db
    416  232 - #3497db
    200  256 - #3497db
    244  292 - #3497db
    100  296 - #3497db
    376  296 - #3497db
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const YELLOW: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #f1c310
    188  100 - #f1c310
    308  100 - #f1c310
    392  100 - #f1c310
    148  144 - #f1c310
    248  156 - #f1c310
      4  168 - #597c95
    388  168 - #f1c310
    188  188 - #f1c310
    592  188 - #597c95
    120  200 - #f1c310
    328  204 - #f1c310
    264  232 - #f1c310
    416  232 - #f1c310
    200  256 - #f1c310
    244  292 - #f1c310
    100  296 - #f1c310
    376  296 - #f1c310
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const HELD_YELLOW: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #f1c310
    188  100 - #f1c310
    308  100 - #f1c310
    392  100 - #f1c310
    148  144 - #f1c310
    248  156 - #f1c310
      4  168 - #597c95
    388  168 - #f1c310
    188  188 - #f1c310
    592  188 - #597c95
    120  200 - #f1c310
    328  204 - #f1c310
    264  232 - #f1c310
    416  232 - #f1c310
    200  256 - #f1c310
    244  292 - #f1c310
    100  296 - #f1c310
    376  296 - #f1c310
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const SOUGHT_GREEN: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #2fcc73
    188  100 - #2fcc73
    308  100 - #2fcc73
    392  100 - #2fcc73
    148  144 - #2fcc73
    248  156 - #2fcc73
      4  168 - #597c95
    388  168 - #2fcc73
    188  188 - #2fcc73
    592  188 - #597c95
    120  200 - #2fcc73
    328  204 - #2fcc73
    264  232 - #2fcc73
    416  232 - #2fcc73
    200  256 - #2fcc73
    244  292 - #2fcc73
    100  296 - #2fcc73
    376  296 - #2fcc73
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const LOOP_BLUE: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #3497db
    188  100 - #3497db
    308  100 - #3497db
    392  100 - #3497db
    148  144 - #3497db
    248  156 - #3497db
      4  168 - #597c95
    388  168 - #3497db
    188  188 - #3497db
    592  188 - #597c95
    120  200 - #3497db
    328  204 - #3497db
    264  232 - #3497db
    416  232 - #3497db
    200  256 - #3497db
    244  292 - #3497db
    100  296 - #3497db
    376  296 - #3497db
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const LOOP_RED: &str = r"
      4    4 - #597c95
    468    4 - #597c95
    592    4 - #597c95
    104  100 - #e94c3d
    188  100 - #e94c3d
    308  100 - #e94c3d
    392  100 - #e94c3d
    148  144 - #e94c3d
    248  156 - #e94c3d
      4  168 - #597c95
    388  168 - #e94c3d
    188  188 - #e94c3d
    592  188 - #597c95
    120  200 - #e94c3d
    328  204 - #e94c3d
    264  232 - #e94c3d
    416  232 - #e94c3d
    200  256 - #e94c3d
    244  292 - #e94c3d
    100  296 - #e94c3d
    376  296 - #e94c3d
    300  300 - #597c95
    592  368 - #597c95
      4  424 - #597c95
    284  436 - #597c95
    420  444 - #597c95
    152  476 - #597c95
    540  480 - #597c95
    444  588 - #597c95
      4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

/// Plays the fixture. Proves the file opens and decodes, the frames reach the
/// screen with the right colors, stepped time advances them on an exact
/// count, the end fires `on_finish`, a seek lands on its frame and a loop
/// wraps.
#[view]
struct VideoPlayback {
    #[init]
    video: VideoView,
}

impl Setup for VideoPlayback {
    fn setup(self: Weak<Self>) {
        let path = temp_dir().join("hilen-video-playback.mp4");
        write(&path, VIDEO).expect("the fixture video is writable to the temp dir");
        self.video.set_frame((100, 100, 320, 200));
        self.video.set_mode(ImageMode::Fill).set_source(path.to_string_lossy());
    }
}

impl ViewTest for VideoPlayback {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);

        // The first frame shows as soon as it is decoded, before play.
        step_frames(1);
        ensure!(from_main(move || view.video.is_loaded()), "the fixture must open");
        let duration = from_main(move || view.video.duration());
        ensure!(
            (duration - 4.0).abs() < 0.01,
            "the fixture is four seconds long, got {duration}"
        );
        ensure!(
            !from_main(move || view.video.is_playing()),
            "a fresh video is paused"
        );
        check_colors(RED)?;

        let (send, finished) = channel();
        from_main(move || {
            view.video.on_finish.sub(move || {
                if send.send(()).is_err() {
                    error!("the video finished after the test stopped waiting");
                }
            });
            view.video.play();
        });
        ensure!(from_main(move || view.video.is_playing()), "play starts playback");

        // A frame shows half a frame interval early, so green lands at 0.5 s.
        step_frames(SECOND * 3 / 4);
        check_colors(GREEN)?;
        step_frames(SECOND);
        check_colors(BLUE)?;
        step_frames(SECOND);
        check_colors(YELLOW)?;
        ensure!(
            finished.try_recv().is_err(),
            "not finished while a frame still shows"
        );

        step_frames(SECOND + SECOND / 2);
        ensure!(
            finished.try_recv().is_ok(),
            "the video must finish after its last frame"
        );
        ensure!(
            !from_main(move || view.video.is_playing()),
            "a finished video is paused"
        );
        check_colors(HELD_YELLOW)?;

        // A seek while paused shows the frame at the target.
        from_main(move || {
            view.video.seek_to(1.0);
        });
        step_frames(1);
        check_colors(SOUGHT_GREEN)?;
        let position = from_main(move || view.video.position());
        ensure!(
            (position - 1.0).abs() < 0.01,
            "position follows the seek, got {position}"
        );

        // Looping wraps to the first frame and never fires on_finish.
        from_main(move || {
            view.video.set_loop(true).seek_to(2.2).play();
        });
        step_frames(1);
        check_colors(LOOP_BLUE)?;
        step_frames(SECOND * 2);
        check_colors(LOOP_RED)?;
        ensure!(finished.try_recv().is_err(), "a looping video never finishes");
        ensure!(
            from_main(move || view.video.is_playing()),
            "a looping video keeps playing"
        );

        from_main(move || {
            view.video.set_loop(false).pause();
        });
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
