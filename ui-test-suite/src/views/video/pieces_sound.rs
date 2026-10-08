use std::{
    env::temp_dir,
    fs::write,
    path::PathBuf,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Label, Setup, TextAlignment, VideoView, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
    video::{VideoPiece, VideoState},
};

/// 60 frames at 30 per second with a sound track, frame `n` is a flat gray
/// that gets lighter with `n`.
const RAMP: &[u8] = include_bytes!("ramp.mp4");
/// Four solid frames at one per second, with no sound track.
const COLORS: &[u8] = include_bytes!("colors.mp4");

fn temp(name: &str) -> PathBuf {
    temp_dir().join(format!("hilen-video-pieces-sound-{name}.mp4"))
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
    let deadline = Instant::now() + Duration::from_secs(10);
    while !from_main(done) {
        if Instant::now() > deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Plays a list of pieces with sound in real time, muted. The sound of the
/// whole list is 1 stream and its position is the clock, where the machine
/// has a sound output. Proves that such a list plays through its cuts and
/// through a piece with no sound to its end in about its own length, that a
/// seek while it plays goes on from the new place, and that a list that
/// finished plays again from its start. `audio/pieces.rs` in the engine
/// pins the samples at a cut.
#[view]
struct VideoPiecesSound {
    finished: u32,

    #[init]
    title:  Label,
    list:   Label,
    video:  VideoView,
    status: Label,
}

impl VideoPiecesSound {
    fn note(label: Weak<Label>, y: u32, text: &str) {
        label.set_frame((20, y, 560, 30));
        label.set_text_size(18).set_alignment(TextAlignment::Left).set_text(text);
    }
}

impl Setup for VideoPiecesSound {
    fn setup(mut self: Weak<Self>) {
        write(temp("ramp"), RAMP).expect("the fixture video is writable to the temp dir");
        write(temp("colors"), COLORS).expect("the fixture video is writable to the temp dir");

        Self::note(self.title, 50, "a list of pieces with sound, muted");
        Self::note(
            self.list,
            80,
            "ramp 0 to 14, ramp 40 to 54, silent green, ramp 15 to 29",
        );
        self.video.set_frame((20, 120, 320, 180));
        Self::note(self.status, 320, "state: none yet");

        self.video.on_state.val(move |state| {
            self.status.set_text(format!("state: {state:?}"));
        });
        self.video.on_finish.sub(move || self.finished += 1);
        let green = VideoPiece::new(temp("colors").to_string_lossy(), 1.0, 1.5);
        self.video.set_mode(ImageMode::Fill).set_volume(0.0).set_pieces([
            ramp(0, 15),
            ramp(40, 55),
            green,
            ramp(15, 30),
        ]);
    }
}

impl ViewTest for VideoPiecesSound {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the first frame", move || {
            view.video.state() == VideoState::Paused
        })?;
        let duration = from_main(move || view.video.duration());
        ensure!(
            (duration - 2.0).abs() < 0.001,
            "3 pieces of half a second and the silent one are 2 seconds, got {duration}"
        );

        let started = Instant::now();
        from_main(move || {
            view.video.play();
        });
        wait_until("the list to pass its first cut", move || {
            view.video.position() > 0.6
        })?;
        ensure!(
            from_main(move || view.video.state()) == VideoState::Playing,
            "the list plays on after its first cut"
        );
        checkpoint("playing, past the first cut")?;
        wait_until("the end of the list", move || view.finished == 1)?;
        let took = started.elapsed().as_secs_f64();
        ensure!(
            (1.8..4.0).contains(&took),
            "a list of 2 seconds played for {took:.2} seconds"
        );
        let position = from_main(move || view.video.position());
        ensure!(
            (position - 2.0).abs() < 0.05,
            "a finished list stands at its end, at {position}"
        );
        checkpoint("finished at the end of the list")?;

        // Play again from the start, then a seek while it plays into the
        // last piece, which follows the piece with no sound.
        from_main(move || {
            view.video.play();
        });
        wait_until("the list to play again", move || {
            let position = view.video.position();
            position > 0.1 && position < 1.0
        })?;
        from_main(move || {
            view.video.seek_to(1.6);
        });
        let sought = Instant::now();
        wait_until("the second end of the list", move || view.finished == 2)?;
        let took = sought.elapsed().as_secs_f64();
        ensure!(
            (0.3..2.5).contains(&took),
            "the last 0.4 seconds of the list played for {took:.2} seconds"
        );
        checkpoint("finished again after a seek while playing")?;
        Ok(())
    }
}
