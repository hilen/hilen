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
    204  100 - #fbfaf9
    416  124 - #dad1cc
    128  132 - #9f9994
    360  136 - #b0a59b
    336  144 - #a62d42
    260  156 - #dad1ce
    220  160 - #63393d
    192  168 - #a7a3a1
    300  168 - #6d1925
    248  180 - #a44251
    148  188 - #c7bcb5
    204  204 - #745051
    248  208 - #a93045
    104  228 - #36656d
    244  228 - #bc6559
    324  228 - #7b2933
    124  232 - #446d75
    288  232 - #b7774c
    392  244 - #332624
    268  248 - #a50d2e
    136  252 - #a29481
    244  252 - #8e0c22
    120  272 - #5e6645
    112  276 - #716d4e
    344  276 - #422928
    348  280 - #462627
    204  284 - #180405
    312  284 - #0f0102
    112  288 - #91885f
    280  288 - #c3294b
    104  296 - #566241
    592  592 - #597c95
";

const FRAME_16: &str = r"
    416  100 - #fbfaf9
    128  132 - #9f9994
    364  136 - #c3b8af
    284  144 - #a26f71
    244  148 - #d6ccc8
    192  168 - #a7a3a1
    216  180 - #7d3e42
    268  188 - #bb4255
    416  192 - #d7ccc9
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
    108  272 - #566241
    368  276 - #402625
    132  288 - #534d3f
    300  292 - #c62d4f
    212  296 - #8a071d
    256  296 - #89061c
    592  592 - #597c95
";

const FRAME_LAST: &str = r"
    268  100 - #fbfaf9
    416  124 - #dad1cc
    148  136 - #a39c97
    324  148 - #7e242f
    248  156 - #ac4a5a
    236  164 - #650f1c
    192  168 - #a7a3a1
    292  172 - #b52241
    100  176 - #c7bcb5
    252  176 - #d4c9c6
    332  200 - #bfb1ac
    208  208 - #64383d
    256  228 - #cf9d80
    268  228 - #8e4545
    288  228 - #a35c53
    108  236 - #37666e
    132  236 - #37666e
    156  236 - #5f7a7d
    112  240 - #a4744b
    132  244 - #c18564
    144  248 - #8b7254
    416  248 - #170305
    204  260 - #c23150
    308  264 - #900d23
    120  268 - #636a48
    344  276 - #422928
    348  280 - #422928
    256  284 - #2c060b
    112  292 - #636a48
    104  296 - #566241
    292  296 - #a20a2a
    592  592 - #597c95
";

const STEPPED_1: &str = r"
    328  100 - #fbfaf9
    104  128 - #cbc0b9
    392  128 - #b7aea3
    316  144 - #5e1923
    276  148 - #822631
    212  152 - #745051
    228  176 - #94595c
    160  188 - #c7bcb5
    264  188 - #af374b
    224  204 - #640e1b
    324  216 - #9b6868
    108  224 - #37666e
    248  224 - #c58b6e
    284  228 - #ad6b4a
    148  232 - #486b71
    108  240 - #928b86
    120  240 - #b7774c
    260  240 - #cd6669
    392  240 - #1f1211
    112  244 - #9e9482
    204  248 - #b92041
    144  252 - #a4ada4
    124  268 - #566241
    100  272 - #566241
    348  272 - #422928
    348  276 - #422928
    252  284 - #240509
    156  288 - #561720
    212  292 - #9f0b29
    300  296 - #9a0d28
    412  296 - #d09666
    592  592 - #597c95
";

const STEPPED_2: &str = r"
    416  100 - #fbfaf9
    128  132 - #9f9994
    196  144 - #c0b6b0
    280  144 - #af7876
    240  152 - #6a101d
    244  176 - #75363d
    232  184 - #a3635d
    220  196 - #64383d
    264  196 - #ba4154
    292  204 - #210507
    340  212 - #9d6667
    104  220 - #36656d
    272  224 - #cba088
    132  228 - #37666e
    164  228 - #cfc3bd
    248  228 - #ce5c60
    292  232 - #cb8e6c
    120  236 - #b58465
    148  236 - #676d58
    416  236 - #e3dad5
    140  240 - #b97a50
    280  240 - #d48263
    216  248 - #c02648
    276  248 - #a20a2a
    316  256 - #bb2042
    100  268 - #566241
    132  272 - #a79d8b
    172  276 - #5b1f27
    208  288 - #88051b
    296  296 - #b01738
    352  296 - #190406
    592  592 - #597c95
";

const STEPPED_3: &str = r"
    120  100 - #fbfaf9
    392  128 - #b7aea3
    232  144 - #98927b
    284  148 - #a52c41
    164  156 - #c9bab6
    340  160 - #a50d2e
    192  168 - #a7a3a1
    292  200 - #9d2438
    208  212 - #e2d9d4
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
    104  264 - #6a7054
    384  272 - #3f0911
    316  276 - #b3193b
    244  288 - #87041a
    264  292 - #930c24
    160  296 - #59161f
    352  296 - #190406
    592  592 - #597c95
";

const WRAPPED_0: &str = r"
    204  100 - #fbfaf9
    416  124 - #dad1cc
    128  132 - #9f9994
    360  136 - #b0a59b
    336  144 - #a62d42
    260  156 - #dad1ce
    220  160 - #63393d
    192  168 - #a7a3a1
    300  168 - #6d1925
    248  180 - #a44251
    148  188 - #c7bcb5
    204  204 - #745051
    248  208 - #a93045
    104  228 - #36656d
    244  228 - #bc6559
    324  228 - #7b2933
    124  232 - #446d75
    288  232 - #b7774c
    392  244 - #332624
    268  248 - #a50d2e
    136  252 - #a29481
    244  252 - #8e0c22
    120  272 - #5e6645
    112  276 - #716d4e
    344  276 - #422928
    348  280 - #462627
    204  284 - #180405
    312  284 - #0f0102
    112  288 - #91885f
    280  288 - #c3294b
    104  296 - #566241
    592  592 - #597c95
";

const HELD_LAST: &str = r"
    268  100 - #fbfaf9
    416  124 - #dad1cc
    148  136 - #a39c97
    324  148 - #7e242f
    248  156 - #ac4a5a
    236  164 - #650f1c
    192  168 - #a7a3a1
    292  172 - #b52241
    100  176 - #c7bcb5
    252  176 - #d4c9c6
    332  200 - #bfb1ac
    208  208 - #64383d
    256  228 - #cf9d80
    268  228 - #8e4545
    288  228 - #a35c53
    108  236 - #37666e
    132  236 - #37666e
    156  236 - #5f7a7d
    112  240 - #a4744b
    132  244 - #c18564
    144  248 - #8b7254
    416  248 - #170305
    204  260 - #c23150
    308  264 - #900d23
    120  268 - #636a48
    344  276 - #422928
    348  280 - #422928
    256  284 - #2c060b
    112  292 - #636a48
    104  296 - #566241
    292  296 - #a20a2a
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
