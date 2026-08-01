use anyhow::Result;
use parking_lot::Mutex;
use test_engine::{
    refs::Weak,
    ui::{BLUE, Button, Setup, ViewData, ViewTest, ViewTransition, view},
    ui_test::{check_colors, inject_touches},
};

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
             452    4 - #597c95
              24   24 - #ffffff
             152   24 - #ffffff
             216   24 - #ffffff
              76   60 - #000000
             116   60 - #ffffff
             120   60 - #ffffff
              76   64 - #000000
             176   64 - #ffffff
              76   68 - #000000
              92   68 - #ffffff
              28   72 - #ffffff
              76   72 - #000000
              92   72 - #ffffff
             116   72 - #ffffff
             120   72 - #ffffff
              76   76 - #000000
             216   76 - #ffffff
              24  116 - #ffffff
             100  116 - #ffffff
             148  116 - #ffffff
             204  116 - #ffffff
             592  196 - #597c95
             380  236 - #597c95
             220  284 - #597c95
              68  356 - #597c95
             516  396 - #597c95
             300  436 - #597c95
               4  592 - #597c95
             192  592 - #597c95
             404  592 - #597c95
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
