use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Button, Setup, TextDropDown, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_touches},
};

/// A drop down whose open list lies over a button that turned its touch on
/// after the drop down. A tap on a row must pick the row. The list was only
/// drawn in front, the touch went to the view that registered last, so the
/// button under the list took the tap and the pick never changed. Found on
/// the Video page of the demo, where the list opens over a text field.
#[view]
struct DropDownOverButton {
    taps: u32,

    #[init]
    drop:  TextDropDown,
    under: Button,
}

impl Setup for DropDownOverButton {
    fn setup(mut self: Weak<Self>) {
        self.drop.set_frame((20, 20, 240, 40));
        self.drop.set_values(["first", "second", "third"]);

        // After the drop down, the order the bug needs.
        self.under.set_frame((20, 64, 240, 200));
        self.under.set_text("under the list").set_color(BLUE).set_text_color(WHITE);
        self.under.on_tap(move || self.taps += 1);
    }
}

impl ViewTest for DropDownOverButton {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Open the list, then tap its second row, 36 points a row under a
        // gap and a padding of a few points.
        inject_touches("140 40 b\n140 40 e");
        ensure!(from_main(move || view.drop.is_opened()), "the list did not open");
        check_colors(OPEN)?;

        inject_touches("140 124 b\n140 124 e");
        let (value, taps, opened) =
            from_main(move || (view.drop.value().to_string(), view.taps, view.drop.is_opened()));
        ensure!(taps == 0, "the button under the list took the tap");
        ensure!(value == "second", "the tap on the second row picked {value:?}");
        ensure!(!opened, "the list stayed open after the pick");

        // With the list closed the button is a button again.
        inject_touches("140 200 b\n140 200 e");
        let taps = from_main(move || view.taps);
        ensure!(taps == 1, "the button took {taps} taps with the list closed");
        Ok(())
    }
}

/// The open list over the button.
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
