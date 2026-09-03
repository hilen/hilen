use anyhow::Result;
use hilen::{
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::check_colors,
};

const DROID: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/fonts/DroidSansMono.ttf"
));

#[view]
struct StemDarkening {
    #[init]
    plain:  Label,
    darker: Label,
}

impl Setup for StemDarkening {
    fn setup(self: Weak<Self>) {
        let darkened =
            Font::with_variations_darkened("droid-darkened", DROID, &[], 0.5).expect("darkened font");

        self.plain.place().tl(20).size(560, 60);
        self.darker.place().t(90).l(20).size(560, 60);

        for label in [self.plain, self.darker] {
            label.set_text("Ink weight check").set_text_size(32).set_color(WHITE);
        }
        self.plain.set_font(Font::get("DroidSansMono.ttf"));
        self.darker.set_font(darkened);
    }
}

impl ViewTest for StemDarkening {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
              32   20 - #ffffff
             188   40 - #242424
             380   40 - #000000
             440   40 - #686868
             272   48 - #1c1c1c
             328   48 - #9c9c9c
             160   52 - #ffffff
             328   52 - #9c9c9c
             188   56 - #242424
             316   56 - #4c4c4c
             440   56 - #686868
             408   60 - #000000
             292   64 - #ffffff
             576   76 - #ffffff
             152  108 - #1e1e1e
             440  112 - #343434
             240  116 - #010101
             440  116 - #353535
             324  120 - #c5c5c5
             328  120 - #4e4e4e
             408  120 - #5e5e5e
             324  124 - #c5c5c5
             328  124 - #4e4e4e
             372  124 - #ffffff
             188  128 - #131313
             316  128 - #262626
             440  128 - #343434
             296  132 - #010101
              20  148 - #ffffff
             300  456 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
