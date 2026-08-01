use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{BLUE, Button, RED, Setup, ViewData, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct JustView {
    #[init]
    red:  Button,
    blue: Button,
}

impl Setup for JustView {
    fn setup(self: Weak<Self>) {
        self.red.set_color(RED).place().left_half();
        self.blue.set_color(BLUE).place().right_half();
    }
}

impl ViewTest for JustView {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        check_colors(
            r"
               4    4 - #ff0000
             152    4 - #ff0000
             440    4 - #0000e7
             592    4 - #0000e7
             296    8 - #ff0000
             516   72 - #0000e7
             364   84 - #0000e7
             444  148 - #0000e7
             148  152 - #ff0000
             592  152 - #0000e7
               4  156 - #ff0000
             292  156 - #ff0000
             368  224 - #0000e7
             512  224 - #0000e7
             152  296 - #ff0000
             440  296 - #0000e7
             588  296 - #0000e7
               4  300 - #ff0000
             296  300 - #ff0000
              84  372 - #ff0000
             228  376 - #ff0000
             444  440 - #0000e7
               8  444 - #ff0000
             304  444 - #0000e7
             592  444 - #0000e7
             152  448 - #ff0000
             228  516 - #ff0000
             448  584 - #0000e7
             304  588 - #0000e7
               4  592 - #ff0000
             160  592 - #ff0000
             592  592 - #0000e7
            ",
        )?;

        Ok(())
    }
}
