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
    fn setup(self: Weak<Self>) {
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
     124   20 - #ffffff
     196   20 - #ffffff
     256   20 - #ffffff
      40   28 - #000000
      36   32 - #000000
      36   36 - #000000
      52   36 - #000000
      36   40 - #000000
      52   40 - #000000
      72   40 - #ffffff
      36   44 - #000000
      52   44 - #010101
      64   44 - #ffffff
      36   48 - #000000
      40   48 - #ffffff
      52   48 - #010101
      56   48 - #ffffff
     108   56 - #ffffff
     168   56 - #ffffff
     220   56 - #ffffff
     256  140 - #0000e7
     592  140 - #597c95
      20  152 - #0000e7
     136  188 - #0000e7
      20  260 - #0000e7
     172  260 - #0000e7
     256  260 - #0000e7
     480  368 - #597c95
     132  432 - #597c95
     300  552 - #597c95
       4  592 - #597c95
     592  592 - #597c95
";

/// The open list covers the blue view.
const OPEN: &str = r"
       148   20 - #ffffff
       256   20 - #ffffff
        40   28 - #000000
        72   40 - #ffffff
        52   44 - #010101
        40   76 - #00daff
        36   80 - #00daff
       260   80 - #445f72
        52   84 - #00daff
        36   88 - #00daff
        52   88 - #00daff
        72   88 - #ebfcff
       188   88 - #ebfcff
        52   92 - #01daff
        36   96 - #00daff
        36  124 - #010101
       104  124 - #000000
        76  128 - #ffffff
       128  128 - #ffffff
       260  140 - #445f72
        36  152 - #000000
        92  164 - #000000
        56  168 - #010101
       188  180 - #ffffff
       260  184 - #445f72
       592  212 - #597c95
        20  260 - #0000e7
       136  260 - #0000e7
       256  260 - #0000e7
       300  552 - #597c95
         4  592 - #597c95
       592  592 - #597c95
";
