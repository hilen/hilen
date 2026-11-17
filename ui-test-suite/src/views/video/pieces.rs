use std::{
    env::temp_dir,
    fs::write,
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
};

use anyhow::{Result, ensure};
use hilen::{
    AppRunner,
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{ImageMode, Label, Setup, TextAlignment, VideoView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, step_frames},
    video::{VideoPiece, VideoState},
};
use log::error;

/// 60 frames at 30 per second with no sound, so the picture follows the
/// engine clock. Frame `n` is a flat gray that gets lighter with `n`, luma
/// `16 + 3 n`. There is 1 keyframe per second, so a piece that starts in the
/// middle is far from where decoding can start.
const RAMP: &[u8] = include_bytes!("ramp_silent.mp4");
/// Four solid frames at one per second, red, green, blue, yellow, in
/// another size than the ramp.
const COLORS: &[u8] = include_bytes!("colors.mp4");

/// The middle of the video on the canvas.
const MIDDLE: (u32, u32) = (180, 210);

/// A quarter of the time 1 frame of the ramp is on screen, in seconds.
const QUARTER_FRAME: f64 = 0.25 / 30.0;

const FIRST_FRAME: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
     92   64 - #273641
    112   64 - #1b252c
    252   64 - #1b252c
    100   72 - #4a677c
    208   88 - #24323c
    252   88 - #24323c
     52   96 - #415a6c
     88   96 - #3e5667
    108   96 - #25343f
    132   96 - #000000
    156   96 - #304351
    316   96 - #1e2a32
    360   96 - #597c95
    424   96 - #597c95
    492   96 - #597c95
    192  100 - #000001
    120  208 - #222324
    232  208 - #222324
     20  244 - #222324
    336  248 - #222324
    104  332 - #000000
    208  332 - #000001
    592  332 - #597c95
     40  336 - #597c95
     72  336 - #415a6d
    136  336 - #151d22
      4  592 - #597c95
    312  592 - #597c95
    592  592 - #597c95
";
const BEFORE_THE_CUT: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    100   72 - #4a677c
    144   88 - #24323c
    252   88 - #24323c
    360   88 - #24323c
    492   88 - #24323c
     52   96 - #415a6c
     88   96 - #3e5667
    108   96 - #25343f
    192   96 - #597c95
    280   96 - #597c95
    316   96 - #1e2a32
    388   96 - #040506
    444   96 - #000000
    292  136 - #323334
     20  200 - #323334
    156  216 - #323334
    260  236 - #323334
    592  328 - #597c95
     68  332 - #11181d
    124  336 - #425c6f
    156  336 - #597c95
    188  336 - #395060
    304  336 - #1b252d
    332  336 - #597c95
    228  340 - #010102
    280  340 - #000001
      4  592 - #597c95
    328  592 - #597c95
    592  592 - #597c95
";
const AFTER_THE_CUT: &str = r"
    168   60 - #000000
     40   64 - #597c95
     92   68 - #273641
    252   68 - #1b252c
    100   72 - #4a677c
    208   88 - #24323c
    492   88 - #24323c
     52   96 - #415a6c
     88   96 - #3e5667
    140   96 - #597c95
    280   96 - #597c95
    316   96 - #1e2a32
    420   96 - #000000
    460   96 - #0e1317
    360  100 - #010101
    176  212 - #aeafb0
    328  216 - #aeafb0
     20  220 - #aeafb0
    100  252 - #aeafb0
    584  332 - #597c95
     40  336 - #597c95
     72  336 - #415a6d
    124  336 - #2f414e
    172  336 - #202c35
    208  336 - #10171b
    232  336 - #597c95
    288  336 - #394f5f
    292  336 - #394f5f
    312  336 - #000000
      4  592 - #597c95
    264  592 - #597c95
    592  592 - #597c95
";
const OTHER_FILE: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    100   64 - #4a677c
    112   64 - #1b252c
    232   68 - #11181d
    252   68 - #1b252c
    100   72 - #4a677c
    208   88 - #24323c
    260   88 - #24323c
    492   88 - #24323c
     52   96 - #415a6c
     88   96 - #3e5667
    108   96 - #25343f
    156   96 - #304351
    316   96 - #1e2a32
    360   96 - #597c95
    424   96 - #597c95
    236  204 - #3497db
    112  216 - #3497db
    336  248 - #3497db
     20  296 - #3497db
    584  332 - #597c95
     40  336 - #597c95
     84  336 - #090c0f
    116  336 - #597c95
    148  336 - #070a0c
    212  336 - #080b0d
    272  336 - #090c0f
      4  592 - #597c95
    328  592 - #597c95
    592  592 - #597c95
";
const BACK_IN_THE_FIRST_FILE: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    128   64 - #597c95
     92   68 - #273641
    252   68 - #1b252c
    260   88 - #24323c
    492   88 - #24323c
     88   92 - #3e5667
    108   96 - #25343f
    140   96 - #597c95
    192   96 - #597c95
    228   96 - #000000
    280   96 - #597c95
    316   96 - #1e2a32
    420   96 - #000000
     68  100 - #2f424f
    360  100 - #010101
     20  200 - #68696a
    336  204 - #68696a
    212  216 - #68696a
    112  236 - #68696a
     68  332 - #4a677b
    176  332 - #476377
    584  332 - #597c95
    112  336 - #3d5566
    156  336 - #3a5162
    196  336 - #1e2a33
    256  340 - #010102
      4  592 - #597c95
    328  592 - #597c95
    592  592 - #597c95
";
const SOUGHT: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    112   64 - #1b252c
    252   68 - #1b252c
    100   72 - #4a677c
    208   88 - #24323c
    260   88 - #24323c
    492   88 - #24323c
     88   92 - #3e5667
     52   96 - #415a6c
    140   96 - #597c95
    236   96 - #597c95
    280   96 - #597c95
    316   96 - #1e2a32
    420   96 - #000000
    460   96 - #0e1317
    360  100 - #010101
    228  200 - #b5b6b7
    112  208 - #b5b6b7
    336  224 - #b5b6b7
     20  276 - #b5b6b7
    584  332 - #597c95
     44  336 - #2f424f
    104  336 - #597c95
    204  336 - #141b21
    224  336 - #40596c
    272  336 - #2e414e
    324  336 - #000000
    364  564 - #597c95
    136  592 - #597c95
    592  592 - #597c95
";
const REORDERED: &str = r"
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    112   68 - #1b252c
    252   68 - #1b252c
    100   72 - #4a677c
    132   88 - #24323c
    360   88 - #24323c
     52   96 - #415a6c
    108   96 - #25343f
    156   96 - #304351
    216   96 - #597c95
    260   96 - #597c95
    316   96 - #1e2a32
     68  100 - #2f424f
    192  156 - #292a2b
    320  208 - #292a2b
     20  216 - #292a2b
    124  232 - #292a2b
    592  256 - #597c95
     68  332 - #11181d
     40  336 - #597c95
    132  336 - #597c95
    184  336 - #4a677b
    208  336 - #466175
    264  336 - #090c0f
    316  336 - #000000
    364  336 - #000000
    408  340 - #000001
      4  592 - #597c95
    292  592 - #597c95
    592  592 - #597c95
";
const TRIMMED: &str = r"
    592    4 - #597c95
    168   60 - #000000
    232   60 - #11181d
     40   64 - #597c95
    100   64 - #4a677c
    100   68 - #4a677c
    252   68 - #1b252c
    100   72 - #4a677c
     88   88 - #24323c
    132   88 - #24323c
    260   88 - #24323c
     52   96 - #415a6c
    116   96 - #597c95
    156   96 - #304351
    216   96 - #597c95
    252  176 - #303132
    332  208 - #303132
     20  216 - #303132
    176  220 - #303132
     68  332 - #11181d
    400  332 - #000001
    112  336 - #000000
    156  336 - #597c95
    188  336 - #1d2830
    248  336 - #151d22
    328  336 - #344857
    372  336 - #597c95
    220  340 - #000001
    280  340 - #010101
      4  592 - #597c95
    332  592 - #597c95
    592  592 - #597c95
";

fn ramp_path() -> PathBuf {
    temp_dir().join("hilen-video-pieces-ramp.mp4")
}

fn colors_path() -> PathBuf {
    temp_dir().join("hilen-video-pieces-colors.mp4")
}

/// The frames of the ramp from `first` up to `end`, which is not included.
fn ramp(first: u32, end: u32) -> VideoPiece {
    VideoPiece::new(
        ramp_path().to_string_lossy(),
        f64::from(first) / 30.0,
        f64::from(end) / 30.0,
    )
}

/// The frame of the color fixture at 2 seconds, blue, for half a second.
fn blue() -> VideoPiece {
    VideoPiece::new(colors_path().to_string_lossy(), 2.0, 2.5)
}

/// What the video shows in its middle.
#[derive(Debug, PartialEq, Eq)]
enum Shown {
    /// This frame of the ramp.
    Ramp(u32),
    Blue,
    Other(u8, u8, u8),
}

fn shown() -> Result<Shown> {
    let pixel = AppRunner::take_screenshot()?.get_pixel(MIDDLE);
    let (r, g, b) = (pixel.r, pixel.g, pixel.b);
    if r.abs_diff(0x34) <= 3 && g.abs_diff(0x97) <= 3 && b.abs_diff(0xdb) <= 3 {
        return Ok(Shown::Blue);
    }
    if r.abs_diff(g) > 3 || g.abs_diff(b) > 3 {
        return Ok(Shown::Other(r, g, b));
    }
    // The gray is the luma over the 219 steps of limited range. The codec
    // is 1 step off on some frames, the frames are 3 steps apart.
    let luma = (u32::from(g) * 219 + 127) / 255;
    Ok(Shown::Ramp((luma + 1) / 3))
}

/// Plays a list of pieces as 1 video under stepped time. Proves that the
/// list has 1 length and 1 position, that every frame of a piece shows and
/// the frame right after the last one of a piece is the first one of the
/// next piece, across a jump inside 1 file and across 2 files, that the end
/// of the list finishes the video, that a seek goes to a place of the whole
/// list, and that a new list on a paused view keeps the position and never
/// goes through loading.
#[view]
struct VideoPieces {
    states: Vec<VideoState>,

    #[init]
    title:  Label,
    list:   Label,
    video:  VideoView,
    status: Label,
}

impl VideoPieces {
    fn note(label: Weak<Label>, y: u32, text: &str) {
        label.set_frame((20, y, 560, 30));
        label.set_text_size(18).set_alignment(TextAlignment::Left).set_text(text);
    }
}

impl Setup for VideoPieces {
    fn setup(mut self: Weak<Self>) {
        write(ramp_path(), RAMP).expect("the fixture video is writable to the temp dir");
        write(colors_path(), COLORS).expect("the fixture video is writable to the temp dir");

        Self::note(self.title, 50, "a list of pieces plays as 1 video");
        Self::note(
            self.list,
            80,
            "ramp 10 to 15, ramp 50 to 55, blue for 0.5 s, ramp 30 to 35",
        );
        self.video.set_frame((20, 120, 320, 180));
        self.video.on_state.val(move |state| self.states.push(state));
        self.video
            .set_mode(ImageMode::Fill)
            .set_pieces([ramp(10, 16), ramp(50, 56), blue(), ramp(30, 36)]);
        Self::note(self.status, 320, "the first frame, ramp 10");
    }
}

/// Says on screen what the next check expects, then checks it.
fn expect(view: Weak<VideoPieces>, what: &'static str, expected: &Shown) -> Result<()> {
    from_main(move || {
        view.status.set_text(what);
    });
    let shown = shown()?;
    ensure!(shown == *expected, "{what}: the video shows {shown:?}");
    Ok(())
}

fn finished(view: Weak<VideoPieces>) -> Receiver<()> {
    let (send, finished) = channel();
    from_main(move || {
        view.video.on_finish.sub(move || {
            if send.send(()).is_err() {
                error!("the video finished after the test stopped waiting");
            }
        });
    });
    finished
}

impl ViewTest for VideoPieces {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);
        step_frames(1);

        ensure!(from_main(move || view.video.is_loaded()), "the list must open");
        let duration = from_main(move || view.video.duration());
        ensure!(
            (duration - 1.1).abs() < 0.001,
            "the 4 pieces are 1.1 seconds together, got {duration}"
        );
        expect(view, "the first frame, ramp 10", &Shown::Ramp(10))?;
        check_colors(FIRST_FRAME)?;

        // Playback starts a quarter of a frame in. From 0 every frame would
        // be due exactly on a step, and the rounding of the clock would
        // decide on which side of it the frame shows.
        let finished = finished(view);
        from_main(move || {
            view.video.seek_to(QUARTER_FRAME).play();
        });

        // A frame shows half a frame before its time, so frame `k` of the
        // list is on screen from step `2 k - 1` on, for 2 steps. The first
        // 2 pieces are 12 frames. Not one is left out and not one comes
        // twice, also not at the cut after frame 5.
        let two_pieces: Vec<u32> = (10..16).chain(50..56).collect();
        for step in 1..=22_usize {
            step_frames(1);
            let frame = two_pieces[step.div_ceil(2)];
            let shown = shown()?;
            ensure!(
                shown == Shown::Ramp(frame),
                "step {step} of the list shows {shown:?}, not ramp {frame}"
            );
            if step == 10 {
                expect(view, "the last frame before the cut, ramp 15", &Shown::Ramp(15))?;
                check_colors(BEFORE_THE_CUT)?;
            }
            if step == 11 {
                expect(view, "the first frame after the cut, ramp 50", &Shown::Ramp(50))?;
                check_colors(AFTER_THE_CUT)?;
            }
        }

        // The third piece starts 0.4 seconds in, a frame of another file in
        // another size. It stays for its half second.
        step_frames(1);
        expect(view, "the piece of the other file, blue", &Shown::Blue)?;
        check_colors(OTHER_FILE)?;
        step_frames(29);
        expect(view, "the last step of the blue piece", &Shown::Blue)?;

        // The last piece starts 0.9 seconds in, back in the first file.
        let last_piece: Vec<u32> = (30..36).collect();
        for step in 0..12_usize {
            step_frames(1);
            let frame = last_piece[step / 2];
            let shown = shown()?;
            ensure!(
                shown == Shown::Ramp(frame),
                "step {step} of the last piece shows {shown:?}, not ramp {frame}"
            );
            if step == 0 {
                expect(view, "back in the first file, ramp 30", &Shown::Ramp(30))?;
                check_colors(BACK_IN_THE_FIRST_FILE)?;
            }
        }
        ensure!(
            finished.try_recv().is_err(),
            "not finished before the end of the list"
        );
        step_frames(6);
        ensure!(
            finished.try_recv().is_ok(),
            "the video must finish at the end of the list"
        );
        ensure!(
            from_main(move || view.video.state()) == VideoState::Finished,
            "a finished list is in the finished state"
        );

        Self::seeks_and_list_changes(view, &finished)
    }
}

impl VideoPieces {
    /// The second half of the test: seeks, then lists that change while
    /// the view is paused.
    fn seeks_and_list_changes(view: Weak<Self>, finished: &Receiver<()>) -> Result<()> {
        // A seek goes to a place of the whole list: 8 frames in is the
        // third frame of the second piece.
        from_main(move || {
            view.video.seek_to(8.0 / 30.0);
        });
        step_frames(1);
        expect(
            view,
            "a seek to 8 frames into the list, ramp 52",
            &Shown::Ramp(52),
        )?;
        check_colors(SOUGHT)?;
        let position = from_main(move || view.video.position());
        ensure!(
            (position - 8.0 / 30.0).abs() < 0.001,
            "the position follows the seek, got {position}"
        );
        // The place of a cut is the first frame after it.
        from_main(move || {
            view.video.seek_to(0.2);
        });
        step_frames(1);
        expect(view, "a seek to the cut, ramp 50", &Shown::Ramp(50))?;
        from_main(move || {
            view.video.seek_to(8.0 / 30.0);
        });
        step_frames(1);

        // The first 2 pieces change places while the view is paused. The
        // position stays, 8 frames in is now the third frame of the piece
        // that starts at ramp 10.
        let states = from_main(move || view.states.len());
        from_main(move || {
            view.list.set_text("ramp 50 to 55, ramp 10 to 15, blue for 0.5 s");
            view.video.set_pieces([ramp(50, 56), ramp(10, 16), blue()]);
        });
        step_frames(1);
        expect(
            view,
            "the list reordered at the same position, ramp 12",
            &Shown::Ramp(12),
        )?;
        check_colors(REORDERED)?;
        let (position, duration) = from_main(move || (view.video.position(), view.video.duration()));
        ensure!(
            (position - 8.0 / 30.0).abs() < 0.001,
            "the position stays when the list changes, got {position}"
        );
        ensure!(
            (duration - 0.9).abs() < 0.001,
            "the new list is 0.9 seconds long, got {duration}"
        );

        // The first piece loses its first 2 frames, a trim. 8 frames in is
        // now the fifth frame of the second piece.
        from_main(move || {
            view.list.set_text("ramp 52 to 55, ramp 10 to 15");
            view.video.set_pieces([ramp(52, 56), ramp(10, 16)]);
        });
        step_frames(1);
        expect(
            view,
            "the list trimmed at the same position, ramp 14",
            &Shown::Ramp(14),
        )?;
        check_colors(TRIMMED)?;
        let changed = from_main(move || view.states[states..].to_vec());
        ensure!(
            changed.is_empty(),
            "a list that changes while paused stays paused, the state went through {changed:?}"
        );

        // The new list plays on from there to its end, 2 frames further.
        from_main(move || {
            view.video.seek_to(8.0 / 30.0 + QUARTER_FRAME).play();
        });
        step_frames(1);
        expect(view, "playing on in the new list, ramp 15", &Shown::Ramp(15))?;
        step_frames(5);
        ensure!(finished.try_recv().is_ok(), "the new list must finish at its end");

        // A list with no pieces empties the view.
        from_main(move || {
            view.video.set_pieces([]);
        });
        ensure!(
            from_main(move || view.video.state()) == VideoState::Empty,
            "a list with no pieces leaves the view empty"
        );

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
