use anyhow::Result;
use test_engine::{
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
             236    4 - #597c95
             396    4 - #e7e7e7
             592   48 - #e7e7e7
             156  104 - #ffffff
             476  108 - #e7e7e7
             296  116 - #ffffff
               4  188 - #597c95
             112  200 - #ffffff
             456  240 - #010101
             576  248 - #010101
             344  252 - #000000
             204  264 - #010101
             240  264 - #010101
               4  300 - #597c95
             128  308 - #000000
             308  312 - #000000
             160  316 - #010101
             204  316 - #010101
             116  320 - #010101
             412  332 - #000000
             528  332 - #000000
             296  380 - #ffffff
               4  412 - #597c95
             564  420 - #e7e7e7
             440  460 - #e7e7e7
             132  484 - #ffffff
             296  496 - #ffffff
               4  592 - #597c95
             208  592 - #597c95
             428  592 - #e7e7e7
             592  592 - #e7e7e7
        ",
        )?;

        Ok(())
    }
}
