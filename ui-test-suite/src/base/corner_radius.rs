use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{Anchor::Left, BLUE, Container, ImageView, Setup, ViewData, ViewTest, YELLOW, view},
    ui_test::{check_colors, set_record_probe_count},
};

#[view]
struct CornerRadius {
    #[init]
    square: Container,
    image:  ImageView,
    wide:   Container,
}

impl Setup for CornerRadius {
    fn setup(self: Weak<Self>) {
        self.square.set_color(BLUE).set_corner_radius(50);
        self.square.place().size(100, 100).tl(50);

        self.image.set_image("cat.png").set_corner_radius(40);
        self.image.place().size(100, 200).t(50).anchor(Left, self.square, 20);

        self.wide.set_color(YELLOW).set_corner_radius(20);
        self.wide.place().size(200, 100).t(50).anchor(Left, self.image, 20);
    }
}

impl ViewTest for CornerRadius {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        check_rounded_corners()
    }
}

fn check_rounded_corners() -> Result<()> {
    check_colors(
        r"
              96   52 - #0000e7
             216   52 - #e2babb
             236   52 - #deb6b7
             356   52 - #ffff00
             396   52 - #ffff00
             440   52 - #ffff00
             480   56 - #ffff00
             124   60 - #0000e7
             304   60 - #ffff00
              68   64 - #0000e7
             180   68 - #e7c0c5
             252   68 - #ddadad
             228   72 - #e1b3b5
              92   76 - #0000e7
             336   80 - #ffff00
             120   84 - #0000e7
             248   84 - #d4a2a1
             416   84 - #ffff00
              72   88 - #0000e7
             144   88 - #0000e7
             172   88 - #ecc5ca
             216   88 - #e2b8b9
             268   88 - #cfa1a1
             192   92 - #e4bcbd
             304   96 - #ffff00
              52  100 - #0000e7
             100  100 - #0000e7
             244  100 - #d5a2a1
             376  100 - #ffff00
             472  100 - #ffff00
             268  104 - #cc9a99
              76  108 - #0000e7
             180  112 - #e7bfc0
             336  116 - #ffff00
             116  120 - #0000e7
             268  120 - #cb9998
              92  124 - #0000e7
             140  124 - #0000e7
             436  128 - #ffff00
             216  132 - #ccb299
             260  132 - #cb9998
              72  136 - #0000e7
             192  136 - #f0d5ca
             108  144 - #0000e7
             172  144 - #e3bbbc
             480  144 - #ffff00
             264  148 - #c89496
             268  148 - #c89496
             312  148 - #ffff00
             356  148 - #ffff00
             392  148 - #ffff00
             264  152 - #c89496
             256  156 - #c89695
             264  156 - #c89496
             256  160 - #c79594
             264  160 - #c89496
             268  160 - #c89496
             264  164 - #c89496
             188  172 - #eed2c7
             240  172 - #c6a28a
             256  172 - #a78b75
             172  176 - #e0a8a9
             260  176 - #ab8f7a
             268  176 - #c79596
             260  180 - #aa8e79
             212  188 - #cfac96
             180  192 - #f1d2cd
             236  196 - #c9a994
             196  208 - #debdaa
             256  212 - #a0846c
             172  216 - #e0a2a5
             592  216 - #597c95
             216  220 - #d1ac99
             236  220 - #bb9b86
             252  228 - #a98b73
             256  228 - #a7876e
             180  232 - #dea4a3
             212  236 - #d0a18d
               4  240 - #597c95
             252  240 - #a88871
             200  244 - #dfb9ae
             224  244 - #ceab97
             452  296 - #597c95
             300  300 - #597c95
             592  352 - #597c95
               4  368 - #597c95
             300  428 - #597c95
             424  444 - #597c95
             176  448 - #597c95
             548  472 - #597c95
              56  480 - #597c95
             444  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )
}
