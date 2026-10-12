use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, Setup, TextDropDown, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// How deep the view under the list is nested. The list was raised by 10
/// nesting steps, so a view nested deeper than that drew over it.
const DEPTH: usize = 30;

/// A drop down whose open list lies over a deeply nested view, like the
/// view tree in the inspector app. The list must draw over all of it. It
/// was raised by a fixed 10 nesting steps, and the blue boxes below level
/// 10 drew over its rows.
#[view]
struct DropDownOverDeepView {
    #[init]
    drop:  TextDropDown,
    under: Container,
}

impl Setup for DropDownOverDeepView {
    fn setup(mut self: Weak<Self>) {
        self.drop.set_frame((20, 20, 240, 40));
        self.drop.set_values(["first", "second", "third"]);

        self.under.set_frame((20, 64, 240, 200));
        let mut level = self.under;
        for _ in 0..DEPTH {
            let next = level.add_view::<Container>();
            next.set_color(BLUE).place().back();
            level = next;
        }
    }
}

impl ViewTest for DropDownOverDeepView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(CLOSED)?;

        inject_touches("140 40 b\n140 40 e");
        ensure!(from_main(move || view.drop.is_opened()), "the list did not open");
        check_colors(OPEN)?;

        // The second row, 36 points a row under a gap and a padding.
        inject_touches("140 124 b\n140 124 e");
        let value = from_main(move || view.drop.value().to_string());
        ensure!(value == "second", "the tap on the second row picked {value:?}");
        Ok(())
    }
}

/// The closed box over the blue view.
const CLOSED: &str = r"
    144   20 - #ffffff
    256   20 - #ffffff
    56   28 - #000000
    52   32 - #000000
    96   32 - #565656
    60   36 - #505050
    68   36 - #000000
    52   40 - #000000
    60   40 - #505050
    68   40 - #010101
    88   40 - #ffffff
    96   40 - #565656
    60   44 - #505050
    68   44 - #010101
    80   44 - #ffffff
    96   44 - #565656
    52   48 - #000000
    56   48 - #ffffff
    60   48 - #505050
    68   48 - #010101
    72   48 - #ffffff
    180   56 - #ffffff
    228   56 - #ffffff
    592  140 - #597c95
    256  144 - #0000e7
    140  188 - #0000e7
    20  260 - #0000e7
    256  260 - #0000e7
    480  368 - #597c95
    300  552 - #597c95
    4  592 - #597c95
    592  592 - #597c95
";

/// The open list covers the blue view.
const OPEN: &str = r"
    256   20 - #ffffff
    56   28 - #000000
    96   32 - #565656
    60   44 - #505050
    96   44 - #565656
    180   52 - #ffffff
    56   76 - #00daff
    60   84 - #4ae5ff
    68   84 - #00daff
    52   88 - #00daff
    60   92 - #4ae5ff
    96   92 - #4fe5ff
    60   96 - #4ae5ff
    68   96 - #01daff
    260  108 - #445f72
    52  120 - #000000
    72  124 - #e6e6e6
    88  128 - #ffffff
    108  128 - #ffffff
    144  128 - #ffffff
    60  148 - #b4b4b4
    108  148 - #000000
    88  160 - #000000
    260  164 - #445f72
    60  168 - #b4b4b4
    108  168 - #010101
    160  184 - #415b6d
    212  184 - #415b6d
    20  260 - #0000e7
    256  260 - #0000e7
    4  592 - #597c95
    592  592 - #597c95
";
