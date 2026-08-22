use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, ImageView, NoImage, Setup, ViewFrame, ViewTest, WHITE, view},
    ui_test::check_colors,
};

#[view]
struct NineSegment {
    #[init]
    segment: ImageView,
    button:  Button,
}

impl Setup for NineSegment {
    fn setup(mut self: Weak<Self>) {
        self.segment.set_frame((50, 50, 200, 200));

        self.segment.set_resizing_image("button");

        self.button.set_image("cat.png");
        self.button.set_frame((50, 400, 200, 100));
        self.button.set_text("llllllllllll");
        self.button.set_text_size(60);
    }
}

fn check_initial_segment() -> Result<()> {
    check_colors(
        r"
             592    4 - #597c95
              84   56 - #00379e
             212   56 - #00389d
              56   88 - #0039a6
             244  104 - #002470
             144  152 - #041141
             244  160 - #002571
             236  216 - #02123f
              64  228 - #0038a3
             152  248 - #003eaa
             592  372 - #597c95
              52  400 - #ebc8ce
             248  400 - #d7a9ab
              72  428 - #796465
             160  428 - #b99c83
             108  432 - #e0c7b6
             136  432 - #d8c3b2
             192  432 - #c9ad97
             224  432 - #a36855
              96  448 - #2f2826
             148  448 - #aa8b6e
             224  448 - #cb9998
              72  452 - #796563
             120  456 - #e2c0b4
              96  468 - #2e2825
             164  472 - #c9a992
             248  472 - #a0846e
             224  476 - #a58973
             196  480 - #a0856f
              80  496 - #e4bfb7
             140  496 - #d2a794
             248  496 - #bb9b86
        ",
    )
}

fn check_wide_segment(view: Weak<NineSegment>) -> Result<()> {
    from_main(move || {
        view.segment.set_frame((100, 100, 250, 160));
    });

    check_colors(
        r"
             592    4 - #597c95
             216  100 - #1954b3
             300  104 - #00379e
             132  108 - #00369c
             344  160 - #002470
             228  188 - #051240
             344  196 - #02266d
             108  228 - #0036a1
             332  232 - #031340
             140  256 - #003baa
             260  256 - #00389f
              52  400 - #ebc8ce
             248  400 - #d7a9ab
              72  428 - #796465
             160  428 - #b99c83
             108  432 - #e0c7b6
             192  432 - #c9ad97
             224  432 - #a36855
              96  448 - #2f2826
             148  448 - #aa8b6e
             224  448 - #cb9998
              72  452 - #796563
             120  456 - #e2c0b4
              96  468 - #2e2825
             164  472 - #c9a992
             248  472 - #a0846e
             224  476 - #a58973
             196  480 - #a0856f
              80  496 - #e4bfb7
             140  496 - #d2a794
             248  496 - #bb9b86
             592  592 - #597c95
        ",
    )
}

fn check_tall_segment(view: Weak<NineSegment>) -> Result<()> {
    from_main(move || {
        view.segment.set_frame((100, 100, 140, 280));
    });

    check_colors(
        r"
             152  100 - #1952ad
             208  108 - #00379f
             232  132 - #0036a3
             104  148 - #003cab
             200  208 - #051140
             112  256 - #05123f
             592  260 - #597c95
             236  304 - #003699
             108  348 - #0036a1
             164  376 - #00399e
              52  400 - #ebc8ce
             248  400 - #d7a9ab
              72  428 - #796465
             160  428 - #b99c83
             108  432 - #e0c7b6
             136  432 - #d8c3b2
             192  432 - #c9ad97
             224  432 - #a36855
              96  448 - #2f2826
             148  448 - #aa8b6e
             224  448 - #cb9998
              72  452 - #796563
             120  456 - #e2c0b4
              96  468 - #2e2825
             164  472 - #c9a992
             248  472 - #a0846e
             224  476 - #a58973
             196  480 - #a0856f
              80  496 - #e4bfb7
             140  496 - #d2a794
             248  496 - #bb9b86
             568  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             152  100 - #1952ad
             208  108 - #00379f
             232  132 - #0036a3
             104  148 - #003cab
             200  208 - #051140
             112  256 - #05123f
             592  260 - #597c95
             236  304 - #003699
             108  348 - #0036a1
             164  376 - #00399e
              52  400 - #ebc8ce
             248  400 - #d7a9ab
              72  428 - #796465
             160  428 - #b99c83
             108  432 - #e0c7b6
             136  432 - #d8c3b2
             192  432 - #c9ad97
             224  432 - #a36855
              96  448 - #2f2826
             148  448 - #aa8b6e
             224  448 - #cb9998
              72  452 - #796563
             120  456 - #e2c0b4
              96  468 - #2e2825
             164  472 - #c9a992
             248  472 - #a0846e
             224  476 - #a58973
             196  480 - #a0856f
              80  496 - #e4bfb7
             140  496 - #d2a794
             248  496 - #bb9b86
             568  592 - #597c95
        ",
    )
}

fn check_button_resizing_image(mut view: Weak<NineSegment>) -> Result<()> {
    from_main(move || {
        view.button.set_image(NoImage);
        view.button.set_resizing_image("button");
        view.button.set_text_color(WHITE);
    });

    check_colors(
        r"
             388    4 - #597c95
             592    4 - #597c95
             140  108 - #050926
             208  108 - #00379f
             232  132 - #0036a3
             100  156 - #0b45ad
             184  184 - #03113e
             120  212 - #03113f
             236  236 - #003597
             104  264 - #0049c2
             168  276 - #041241
             532  296 - #597c95
             100  316 - #0b48b6
             236  332 - #00369d
             132  372 - #0037a2
             188  376 - #00389d
              96  428 - #cccfd8
             116  428 - #3f4a6f
             156  428 - #ffffff
             212  428 - #15214d
             184  436 - #ffffff
             116  440 - #3f4b70
             212  444 - #13214a
              96  452 - #cccfd8
             212  464 - #14214c
              68  468 - #ffffff
              96  468 - #ccced8
             116  468 - #3f4a6e
             156  468 - #ffffff
             244  472 - #00369f
             180  496 - #003caa
             592  592 - #597c95
        ",
    )
}

impl ViewTest for NineSegment {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_initial_segment()?;
        check_wide_segment(view)?;
        check_tall_segment(view)?;
        check_button_resizing_image(view)?;

        Ok(())
    }
}
