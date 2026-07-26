use anyhow::Result;
use test_engine::{
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
             376    4 - #597c95
             592    4 - #597c95
             168   52 - #0039a2
              76   68 - #031140
             152  144 - #051343
              60  168 - #04123f
             240  168 - #031340
             128  240 - #051243
             300  300 - #597c95
             592  300 - #597c95
              52  404 - #ebc6cd
             104  404 - #ebc3c4
             160  404 - #dfb7b8
             232  404 - #daacae
             132  408 - #e6bebe
             208  420 - #d49e9e
             248  428 - #ce9898
             156  432 - #000000
              76  440 - #000000
             128  440 - #000000
             156  460 - #000000
             192  460 - #c5a189
             236  464 - #a38875
             224  472 - #a2836e
              68  480 - #e6b4b3
             228  480 - #9c806a
             184  484 - #ba9986
             212  488 - #ab8d75
             116  496 - #dcb4aa
             160  496 - #ceab97
             240  496 - #b79b85
             592  592 - #597c95
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
             128  116 - #04133f
             320  116 - #03123f
             224  172 - #04113f
             340  192 - #041140
             224  248 - #061443
             148  256 - #00399d
             300  256 - #00379e
             300  300 - #597c95
             592  300 - #597c95
              52  404 - #ebc6cd
             104  404 - #ebc3c4
             160  404 - #dfb7b8
             232  404 - #daacae
             208  420 - #d49e9e
             248  428 - #ce9898
             156  432 - #000000
              76  440 - #000000
             128  440 - #000000
             448  448 - #597c95
             156  460 - #000000
             192  460 - #c5a189
             236  464 - #a38875
             224  472 - #a2836e
              68  480 - #e6b4b3
             228  480 - #9c806a
             184  484 - #ba9986
             212  488 - #ab8d75
             116  496 - #dcb4aa
             160  496 - #ceab97
             240  496 - #b79b85
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
             380    4 - #597c95
             592    4 - #597c95
             128  116 - #04133f
             228  152 - #03103d
             112  196 - #02103e
             176  244 - #041340
             112  300 - #04123f
             300  300 - #597c95
             592  300 - #597c95
             148  376 - #00399d
             192  376 - #00399b
              52  404 - #ebc6cd
             104  404 - #ebc3c4
             248  404 - #d9abad
             164  416 - #dfb1b1
             208  420 - #d49e9e
             248  436 - #ca9897
              76  440 - #000000
             128  440 - #000000
             156  460 - #000000
             192  460 - #c5a189
             116  464 - #000000
             236  464 - #a38875
             224  472 - #a2836e
              68  480 - #e6b4b3
             228  480 - #9c806a
             184  484 - #ba9986
             212  488 - #ab8d75
             112  496 - #ddb8af
             160  496 - #ceab97
             240  496 - #b79b85
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             380    4 - #597c95
             592    4 - #597c95
             128  116 - #04133f
             228  152 - #03103d
             112  196 - #02103e
             176  244 - #041340
             112  300 - #04123f
             300  300 - #597c95
             592  300 - #597c95
             148  376 - #00399d
             192  376 - #00399b
              52  404 - #ebc6cd
             104  404 - #ebc3c4
             248  404 - #d9abad
             164  416 - #dfb1b1
             208  420 - #d49e9e
             248  436 - #ca9897
              76  440 - #000000
             128  440 - #000000
             156  460 - #000000
             192  460 - #c5a189
             116  464 - #000000
             236  464 - #a38875
             224  472 - #a2836e
              68  480 - #e6b4b3
             228  480 - #9c806a
             184  484 - #ba9986
             212  488 - #ab8d75
             112  496 - #ddb8af
             160  496 - #ceab97
             240  496 - #b79b85
             592  592 - #597c95
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
               4    4 - #597c95
             384    4 - #597c95
             592    4 - #597c95
             128  116 - #04133f
             228  144 - #03123f
             592  152 - #597c95
             172  176 - #041241
             448  180 - #597c95
             112  196 - #02103e
             228  204 - #04103f
             180  240 - #051342
             112  296 - #03123f
             300  300 - #597c95
             592  300 - #597c95
             224  304 - #03103c
             168  320 - #041242
             148  376 - #00399d
             192  376 - #00399b
             448  424 - #597c95
              76  428 - #ffffff
             156  428 - #ffffff
             196  428 - #ffffff
             116  436 - #ffffff
             156  444 - #ffffff
             196  444 - #ffffff
              76  452 - #ffffff
             116  464 - #ffffff
             156  464 - #ffffff
             196  464 - #ffffff
              60  472 - #03113d
             360  592 - #597c95
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
