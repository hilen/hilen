use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{LIGHT_GRAY, Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::helpers::check_colors,
};

#[view]
pub struct TextOccclusion {
    #[init]
    label_below: Label,
    label_above: Label,
}

impl Setup for TextOccclusion {
    fn setup(self: Weak<Self>) {
        self.label_below
            .set_color(WHITE)
            .set_text_size(100)
            .set_text("OOOOOOOO")
            .place()
            .size(400, 400)
            .center();

        self.label_above
            .set_text_size(140)
            .set_text("A A A A A")
            .set_color(LIGHT_GRAY)
            .place()
            .right_half();
    }
}

impl ViewTest for TextOccclusion {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
               4    4 - #597c95
             592    4 - #e7e7e7
             296   20 - #597c95
             448   84 - #e7e7e7
             144   96 - #597c95
               4  160 - #597c95
             584  164 - #e7e7e7
             276  168 - #ffffff
             100  180 - #ffffff
             324  248 - #000000
             576  248 - #000000
             444  252 - #000000
             196  264 - #000000
             264  264 - #000000
             112  272 - #000000
             224  300 - #000000
             284  316 - #010101
             592  320 - #010101
             128  332 - #000000
             196  332 - #000000
             364  344 - #010101
             488  344 - #010101
             100  384 - #ffffff
             192  440 - #ffffff
             424  460 - #e7e7e7
             100  496 - #ffffff
             280  496 - #ffffff
             592  500 - #e7e7e7
               4  592 - #597c95
             192  592 - #597c95
             328  592 - #e7e7e7
             436  592 - #e7e7e7
        ",
        )?;

        Ok(())
    }
}
