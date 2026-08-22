use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLUE, Button, Setup, ViewData, ViewTest, ViewTransition, view},
    ui_test::{check_colors, inject_touches},
};
use parking_lot::Mutex;

static ACTIONS: Mutex<Vec<&str>> = Mutex::new(vec![]);

#[view]
struct Transition {
    #[init]
    to_blue: Button,
}

impl Setup for Transition {
    fn setup(self: Weak<Self>) {
        self.to_blue.set_text("To Blue");
        self.to_blue.place().tl(20).size(200, 100);
        self.to_blue.add_transition::<Self, BlueView>();
    }
}

impl ViewTransition<BlueView> for Transition {
    fn transition_to(self: Weak<Self>, _target: &mut BlueView) {
        ACTIONS.lock().push("Transition callback");
    }
}

#[view]
struct BlueView {}

impl Setup for BlueView {
    fn setup(self: Weak<Self>) {
        self.set_color(BLUE);
        ACTIONS.lock().push("Blue setup");
    }
}

impl ViewTest for Transition {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             160   20 - #ffffff
             216   20 - #ffffff
              20   24 - #ffffff
              72   60 - #585858
              80   60 - #585858
             124   60 - #000000
             116   64 - #a4a4a4
             120   64 - #ffffff
              76   68 - #000000
              92   68 - #ffffff
             116   68 - #000000
             128   68 - #ffffff
             140   68 - #909090
             152   68 - #000000
             164   68 - #ffffff
              76   72 - #000000
             100   72 - #000000
             116   72 - #a4a4a4
             140   72 - #909090
             168   72 - #000000
              76   76 - #000000
              92   76 - #ffffff
             116   76 - #a4a4a4
             152   76 - #000000
             164   80 - #000000
              20  116 - #ffffff
             216  116 - #ffffff
             592  180 - #597c95
              68  356 - #597c95
             300  436 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        inject_touches(
            "
            142  88   b
            142  87   e

        ",
        );

        check_colors(
            r"
               4    4 - #0000e7
             444    4 - #0000e7
             592    4 - #0000e7
             296    8 - #0000e7
             148   12 - #0000e7
             228   84 - #0000e7
              12  148 - #0000e7
             444  152 - #0000e7
             592  152 - #0000e7
             156  156 - #0000e7
             300  156 - #0000e7
              84  228 - #0000e7
             228  228 - #0000e7
             372  228 - #0000e7
               8  296 - #0000e7
             448  296 - #0000e7
             156  300 - #0000e7
             300  300 - #0000e7
             592  300 - #0000e7
             228  372 - #0000e7
             372  372 - #0000e7
             516  372 - #0000e7
               4  444 - #0000e7
             152  444 - #0000e7
             444  444 - #0000e7
             296  448 - #0000e7
             588  448 - #0000e7
             448  588 - #0000e7
               4  592 - #0000e7
             152  592 - #0000e7
             300  592 - #0000e7
             592  592 - #0000e7
        ",
        )?;

        assert_eq!(ACTIONS.lock().as_slice(), &["Transition callback", "Blue setup"]);

        ACTIONS.lock().clear();

        Ok(())
    }
}
