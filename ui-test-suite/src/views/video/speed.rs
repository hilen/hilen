use std::{env::temp_dir, fs::write};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, step_frames},
};

/// The fixture of the playback test: four solid frames at one per second,
/// red, green, blue, yellow, no sound, so the picture follows the engine
/// clock and stepped time drives it.
const VIDEO: &[u8] = include_bytes!("colors.mp4");

/// One second on the 60 fps stepped timeline.
const SECOND: u32 = 60;

/// Plays the fixture at double and at half speed. Proves the position moves
/// by the speed times the clock, a speed change mid way keeps the position,
/// and a 4 second video at double speed finishes after 2 seconds. The sound
/// side, the pitch kept at every speed, is pinned by
/// `speed_changes_the_length_and_keeps_the_pitch` in the engine.
#[view]
struct VideoSpeed {
    #[init]
    title: Label,
    video: VideoView,
    state: Label,
}

impl VideoSpeed {
    fn describe(self: Weak<Self>) {
        self.state.set_text(format!(
            "speed {}, position {:.2} s",
            self.video.speed(),
            self.video.position()
        ));
    }
}

impl Setup for VideoSpeed {
    fn setup(self: Weak<Self>) {
        let path = temp_dir().join("hilen-video-speed.mp4");
        write(&path, VIDEO).expect("the fixture video is writable to the temp dir");
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("a 4 second video at double and half speed");
        self.video.set_frame((140, 100, 320, 200));
        self.video.set_mode(ImageMode::Fill).set_source(path.to_string_lossy());
        self.state.set_frame((20, 320, 560, 40));
    }
}

impl ViewTest for VideoSpeed {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);
        step_frames(1);
        ensure!(from_main(move || view.video.is_loaded()), "the fixture must open");

        let position = move || {
            from_main(move || {
                view.describe();
                view.video.position()
            })
        };

        from_main(move || {
            view.video.set_speed(2.0).play();
        });
        step_frames(SECOND / 2);
        let at = position();
        ensure!(
            (at - 1.0).abs() < 0.05,
            "half a second at double speed is 1 s in, got {at}"
        );
        checkpoint("double speed, 1 s in after half a second")?;

        // Slower from here on: the position stays, then moves at the new speed.
        from_main(move || {
            view.video.set_speed(0.5);
        });
        let kept = position();
        ensure!(
            (kept - at).abs() < 0.05,
            "a speed change keeps the position, {at} to {kept}"
        );
        step_frames(SECOND);
        let at = position();
        ensure!(
            (at - 1.5).abs() < 0.05,
            "a second at half speed adds 0.5 s, got {at}"
        );
        checkpoint("half speed, 1.5 s in")?;

        // Back to double: the 2.5 s that are left take 1.25 s.
        from_main(move || {
            view.video.set_speed(2.0);
        });
        step_frames(SECOND);
        ensure!(
            from_main(move || view.video.is_playing()),
            "still playing after 1 s"
        );
        step_frames(SECOND / 2);
        ensure!(
            !from_main(move || view.video.is_playing()),
            "the video is over after 1.25 s more"
        );
        position();
        checkpoint("finished early, the speed made it shorter")?;

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
