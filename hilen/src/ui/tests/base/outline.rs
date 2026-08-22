use anyhow::Result;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, GREEN, RED, YELLOW},
    ui::{Anchor::Left, Container, ImageView, Setup, ViewData, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct Outline {
    #[init]
    square: Container,
    image:  ImageView,
    wide:   Container,
}

impl Setup for Outline {
    fn setup(self: Weak<Self>) {
        self.square.set_color(BLUE).set_border_width(10).set_border_color(RED);
        self.square.place().size(100, 100).tl(50);

        self.image.set_image("cat.png").set_border_width(5).set_border_color(GREEN);
        self.image.place().size(100, 200).t(50).anchor(Left, self.square, 20);

        self.wide.set_color(YELLOW).set_border_width(25).set_border_color(BLUE);
        self.wide.place().size(200, 100).t(50).anchor(Left, self.image, 20);
    }
}

impl ViewTest for Outline {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             120   52 - #ff0000
             260   56 - #dbadaf
              64   64 - #0000e7
             176   68 - #eac3c8
             340   76 - #ffff00
             460   76 - #ffff00
             220   84 - #e2b4b6
             400   88 - #ffff00
              52  104 - #ff0000
             100  112 - #0000e7
             172  112 - #00ff00
             268  116 - #00ff00
             320  120 - #ffff00
             216  132 - #ccb299
              52  148 - #ff0000
             148  148 - #ff0000
             376  148 - #0000e7
             488  148 - #0000e7
             256  156 - #c89695
             240  172 - #c6a28a
             256  172 - #a78b75
             268  180 - #00ff00
             188  188 - #e9cdc2
             256  212 - #a0846c
             216  220 - #d1ac99
             236  220 - #bb9b86
             176  240 - #dda3a2
             252  240 - #a88871
             300  300 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        from_main(move || {
            view.square.set_corner_radius(15);
            view.image.set_corner_radius(25);
            view.wide.set_corner_radius(40);
        });

        check_colors(
            r"
             116   52 - #ff0000
             232   52 - #00ff00
              56   56 - #ff0000
             480   68 - #0000e7
             172   72 - #00ff00
             388   76 - #ffff00
             316   92 - #ffff00
             260   96 - #cf9c9b
             436  100 - #ffff00
             376  120 - #ffff00
             176  124 - #e4bcbd
             216  132 - #ccb299
             476  136 - #0000e7
              84  148 - #ff0000
             344  148 - #0000e7
             408  148 - #0000e7
             260  156 - #c99597
             256  160 - #c79594
             188  172 - #eed2c7
             240  172 - #c6a28a
             260  180 - #aa8e79
             256  212 - #a0846c
             176  220 - #e2acaa
             236  220 - #bb9b86
             252  228 - #a98b73
             264  236 - #00ff00
             256  244 - #00ff00
             208  248 - #00ff00
             592  348 - #597c95
             300  540 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        // crate::ui_test::record_ui_test();

        Ok(())
    }
}
