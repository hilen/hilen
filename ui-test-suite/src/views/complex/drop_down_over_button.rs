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
