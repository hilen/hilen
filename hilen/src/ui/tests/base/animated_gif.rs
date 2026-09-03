use std::sync::mpsc::channel;

use anyhow::{Result, ensure};
use log::error;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::Clock,
    ui::{AnimatedImage, ImageMode, Setup, ViewFrame, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

/// The kukareker bug report rooster, the karkas `bug-report-animation.mp4`
/// downscaled to 160x100 at 5 fps, 33 frames of 200 ms each.
const GIF: &[u8] = include_bytes!("rooster.gif");
const FRAME_COUNT: usize = 33;
const LAST: usize = FRAME_COUNT - 1;

/// 200 ms per frame on the 60 fps stepped timeline is exactly 12 frames.
const FRAMES_PER_STEP: u32 = 12;

const FRAME_0: &str = r"
    108  100 - #d2d4cb
    404  100 - #d8d7ce
    312  104 - #fbfaf9
    224  144 - #a6a09d
    336  144 - #a62d42
    192  148 - #c1b8b5
    300  160 - #670f1c
    244  168 - #6a101d
    220  196 - #774244
    252  204 - #c84d5c
    284  208 - #950d26
    104  228 - #36656d
    244  228 - #bc6559
    324  228 - #7b2933
    124  232 - #446d75
    212  232 - #6d3339
    268  232 - #a05851
    392  244 - #332624
    152  248 - #c58a6c
    296  248 - #a50c2d
    136  252 - #a29481
    232  272 - #be2345
    124  276 - #566241
    344  276 - #422928
    112  280 - #c09d86
    348  280 - #462627
    312  284 - #0f0102
    280  288 - #c3294b
    112  292 - #7e7655
    104  296 - #566241
    192  296 - #180405
    592  592 - #597c95
";

const FRAME_16: &str = r"
    336  100 - #c2bcbc
    408  100 - #e8e5df
    128  132 - #9f9994
    284  144 - #a26f71
    244  148 - #d6ccc8
    192  168 - #a7a3a1
    216  180 - #7d3e42
    268  188 - #bb4255
    416  188 - #dfd6d2
    328  200 - #1e0507
    108  208 - #31626a
    120  212 - #33636b
    216  212 - #ab5f52
    156  216 - #5f7a7d
    112  224 - #847351
    128  224 - #b6764a
    140  224 - #d8905e
    244  236 - #e49964
    276  236 - #190406
    264  244 - #da8e62
    128  248 - #566241
    224  252 - #82061b
    324  252 - #900d23
    104  256 - #827d57
    416  260 - #110102
    368  276 - #402625
    132  288 - #534d3f
    300  292 - #c62d4f
    412  292 - #aca483
    212  296 - #8a071d
    256  296 - #89061c
    592  592 - #597c95
";

const FRAME_LAST: &str = r"
    108  100 - #e0ddd2
    416  100 - #eeeae1
    196  104 - #fbfaf9
    324  148 - #7e242f
    192  156 - #b9a9a6
    224  156 - #844f55
    248  156 - #ac4a5a
    264  156 - #c7b0af
    416  196 - #c8beb7
    212  204 - #64383d
    324  208 - #1c0406
    248  224 - #b97b64
    268  228 - #8e4545
    288  228 - #a35c53
    108  236 - #37666e
    132  236 - #37666e
    156  236 - #5f7a7d
    260  236 - #cba088
    320  244 - #620e1b
    144  248 - #8b7254
    100  256 - #94897b
    276  268 - #a20a2a
    224  272 - #c62d4f
    304  272 - #951329
    176  276 - #4b0e18
    344  276 - #422928
    104  280 - #b6947a
    348  280 - #422928
    112  292 - #636a48
    416  292 - #180405
    268  296 - #bd2244
    592  592 - #597c95
";

const STEPPED_1: &str = r"
    104  100 - #e5e1dd
    408  100 - #c7bebe
    168  104 - #fbfaf9
    392  128 - #b7aea3
    248  144 - #b9b0a6
    284  144 - #8a696c
    316  144 - #5e1923
    212  152 - #745051
    228  176 - #94595c
    276  184 - #b11839
    244  204 - #b6384e
    320  204 - #1b0406
    204  208 - #492e2d
    416  212 - #c5bcb6
    108  224 - #37666e
    248  224 - #c58b6e
    284  228 - #ad6b4a
    148  232 - #486b71
    108  240 - #928b86
    120  240 - #b7774c
    260  240 - #cd6669
    112  244 - #9e9482
    336  248 - #782834
    204  252 - #ba1f41
    124  268 - #566241
    104  272 - #566241
    156  288 - #561720
    212  292 - #9f0b29
    264  292 - #88081d
    312  296 - #1b0406
    412  296 - #d09666
    592  592 - #597c95
";

const STEPPED_2: &str = r"
    208  100 - #debcc1
    416  104 - #fbfaf9
    128  132 - #9f9994
    280  144 - #af7876
    352  152 - #e1d8d4
    192  164 - #a39e9b
    244  176 - #75363d
    232  184 - #a3635d
    220  196 - #64383d
    264  196 - #ba4154
    292  204 - #210507
    340  212 - #9d6667
    104  220 - #36656d
    272  224 - #cba088
    164  228 - #cfc3bd
    248  228 - #ce5c60
    148  232 - #36656d
    292  232 - #cb8e6c
    120  236 - #b58465
    128  236 - #9e7148
    140  236 - #6c705b
    216  248 - #c02648
    276  248 - #a20a2a
    360  264 - #49141b
    124  272 - #5c6544
    172  276 - #5b1f27
    108  288 - #566241
    208  288 - #88051b
    416  288 - #190406
    248  296 - #bc2143
    320  296 - #a00e2c
    592  592 - #597c95
";

const STEPPED_3: &str = r"
    120  100 - #f3f0ef
    308  100 - #e5bec5
    392  128 - #b7aea3
    232  144 - #98927b
    284  148 - #a52c41
    340  160 - #a50d2e
    192  164 - #a39e9b
    412  176 - #ddd4d1
    224  196 - #64383d
    292  200 - #9d2438
    280  216 - #c18f7b
    108  220 - #36656d
    152  224 - #cdc2bb
    300  224 - #cba088
    360  224 - #71323a
    148  228 - #5b777c
    264  228 - #cf7162
    128  232 - #31626a
    284  236 - #e49964
    304  236 - #160305
    324  236 - #e49964
    380  236 - #332624
    148  244 - #8a825c
    232  252 - #c62d4f
    136  260 - #c07e51
    100  276 - #736749
    316  276 - #b3193b
    244  288 - #87041a
    264  292 - #930c24
    152  296 - #3b1c1e
    352  296 - #190406
    592  592 - #597c95
";

const WRAPPED_0: &str = r"
    108  100 - #d2d4cb
    404  100 - #d8d7ce
    312  104 - #fbfaf9
    224  144 - #a6a09d
    336  144 - #a62d42
    192  148 - #c1b8b5
    300  160 - #670f1c
    244  168 - #6a101d
    220  196 - #774244
    252  204 - #c84d5c
    284  208 - #950d26
    104  228 - #36656d
    244  228 - #bc6559
    324  228 - #7b2933
    124  232 - #446d75
    212  232 - #6d3339
    268  232 - #a05851
    392  244 - #332624
    152  248 - #c58a6c
    296  248 - #a50c2d
    136  252 - #a29481
    232  272 - #be2345
    124  276 - #566241
    344  276 - #422928
    112  280 - #c09d86
    348  280 - #462627
    312  284 - #0f0102
    280  288 - #c3294b
    112  292 - #7e7655
    104  296 - #566241
    192  296 - #180405
    592  592 - #597c95
";

const HELD_LAST: &str = r"
    108  100 - #e0ddd2
    416  100 - #eeeae1
    196  104 - #fbfaf9
    324  148 - #7e242f
    192  156 - #b9a9a6
    224  156 - #844f55
    248  156 - #ac4a5a
    264  156 - #c7b0af
    416  196 - #c8beb7
    212  204 - #64383d
    324  208 - #1c0406
    248  224 - #b97b64
    268  228 - #8e4545
    288  228 - #a35c53
    108  236 - #37666e
    132  236 - #37666e
    156  236 - #5f7a7d
    260  236 - #cba088
    320  244 - #620e1b
    144  248 - #8b7254
    100  256 - #94897b
    276  268 - #a20a2a
    224  272 - #c62d4f
    304  272 - #951329
    176  276 - #4b0e18
    344  276 - #422928
    104  280 - #b6947a
    348  280 - #422928
    112  292 - #636a48
    416  292 - #180405
    268  296 - #bd2244
    592  592 - #597c95
";

/// Plays a gif. Proves the decode and the per-frame textures render the right
/// pixels, and that stepped time advances the frames on an exact count and a
/// loop count stops on the last frame.
#[view]
struct AnimatedGif {
    #[init]
    anim: AnimatedImage,
}

impl Setup for AnimatedGif {
    fn setup(self: Weak<Self>) {
        self.anim.set_mode(ImageMode::Fill);
        self.anim.set_frame((100, 100, 320, 200));
        self.anim.set_gif(GIF).expect("failed to decode fixture gif");
        self.anim.pause();
        self.anim.show_frame(0);
    }
}

/// Advance one gif frame worth of stepped time and check the gif landed on
/// `expected`.
fn step_to(view: Weak<AnimatedGif>, expected: usize) -> Result<()> {
    step_frames(FRAMES_PER_STEP);
    let current = from_main(move || view.anim.current_frame());
    ensure!(
        current == expected,
        "after {expected} frames worth the gif should be on frame {expected}, got {current}"
    );
    Ok(())
}

impl ViewTest for AnimatedGif {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(
            from_main(move || view.anim.frame_count()) == FRAME_COUNT,
            "the fixture gif must decode to {FRAME_COUNT} frames"
        );

        // Decode and render: first, middle and last frame each show their own
        // picture.
        check_colors(FRAME_0)?;

        from_main(move || {
            view.anim.show_frame(16);
        });
        check_colors(FRAME_16)?;

        from_main(move || {
            view.anim.show_frame(LAST);
        });
        check_colors(FRAME_LAST)?;

        // Auto advance under stepped time, and the loop count stopping on the
        // last frame.
        let (send, finished) = channel();
        from_main(move || {
            Clock::enter_stepped();
            view.anim.on_finish.sub(move || {
                if send.send(()).is_err() {
                    error!("gif finished after the test stopped waiting");
                }
            });
            view.anim.set_loop(2);
            view.anim.show_frame(0);
            view.anim.play();
        });

        ensure!(
            from_main(move || view.anim.current_frame()) == 0,
            "should start on frame 0"
        );

        // One frame worth of stepped time per gif frame, each landing on the
        // next picture.
        step_to(view, 1)?;
        check_colors(STEPPED_1)?;

        step_to(view, 2)?;
        check_colors(STEPPED_2)?;

        step_to(view, 3)?;
        check_colors(STEPPED_3)?;

        // Jump near the end, one frame worth lands on the last frame and one
        // more wraps back to 0, the first loop is done.
        from_main(move || {
            view.anim.show_frame(LAST - 1);
        });
        step_frames(FRAMES_PER_STEP);
        ensure!(
            from_main(move || view.anim.current_frame()) == LAST,
            "one frame worth from second to last must land on the last frame"
        );
        step_frames(FRAMES_PER_STEP);
        ensure!(
            from_main(move || view.anim.current_frame()) == 0,
            "the gif should wrap to frame 0 after the first loop"
        );
        ensure!(
            finished.try_recv().is_err(),
            "one loop of two must not finish yet"
        );
        check_colors(WRAPPED_0)?;

        // Same again through the second loop, then it stops on the last frame.
        from_main(move || {
            view.anim.show_frame(LAST - 1);
        });
        step_frames(FRAMES_PER_STEP * 2);
        ensure!(
            finished.try_recv().is_ok(),
            "the gif must finish after its loop count"
        );
        ensure!(
            !from_main(move || view.anim.is_playing()),
            "the gif must stop playing after its loop count"
        );
        ensure!(
            from_main(move || view.anim.current_frame()) == LAST,
            "a finished gif holds on its last frame"
        );
        // Held on the last frame, the same picture the third check pinned.
        check_colors(HELD_LAST)?;

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
