use anyhow::Result;

use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::color::{BLUE, GREEN, RED},
    ui::{
        Anchor::{Bot, Top},
        Container, Setup, ViewData, ViewTest, view,
    },
    ui_test::check_colors,
};

#[view]
pub(crate) struct AnchorLayoutTest {
    #[init]
    top:    Container,
    bot:    Container,
    target: Container,
}

impl Setup for AnchorLayoutTest {
    fn setup(self: Weak<Self>) {
        self.top.set_color(RED).place().tl(20).size(50, 50);
        self.bot.set_color(GREEN).place().bl(20).size(50, 50);
        self.target
            .set_color(BLUE)
            .place()
            .anchor(Top, self.top, 20)
            .l(20)
            .anchor(Bot, self.bot, 20)
            .w(200);
    }
}

impl ViewTest for AnchorLayoutTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             380    4 - #597c95
             592    4 - #597c95
              40   24 - #ff0000
              68   24 - #ff0000
              24   28 - #ff0000
              24   44 - #ff0000
              52   44 - #ff0000
              68   52 - #ff0000
              36   68 - #ff0000
              64   68 - #ff0000
              24   92 - #0000e7
             216  136 - #0000e7
             448  180 - #597c95
             100  204 - #0000e7
              24  252 - #0000e7
             176  300 - #0000e7
             300  300 - #597c95
             592  300 - #597c95
              60  336 - #0000e7
             448  420 - #597c95
             108  432 - #0000e7
             216  448 - #0000e7
              24  532 - #00ff00
              48  532 - #00ff00
              68  532 - #00ff00
              48  552 - #00ff00
              28  556 - #00ff00
              44  572 - #00ff00
              24  576 - #00ff00
              68  576 - #00ff00
             376  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        // record_ui_test();

        Ok(())
    }
}
