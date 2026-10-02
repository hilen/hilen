use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, Label, Setup, TextAlignment, TextDropDown, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// An open list closes on a press anywhere outside its box and its panel,
/// so 2 lists are never open together and a list never stays over a form
/// the user went on with. A press on a row still picks, and the button the
/// outside press landed on still gets its tap.
#[view]
struct DropDownOutsideTap {
    taps: u32,

    #[init]
    left_title:  Label,
    left:        TextDropDown,
    right_title: Label,
    right:       TextDropDown,
    button:      Button,
    state:       Label,
}

impl Setup for DropDownOutsideTap {
    fn setup(mut self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, x: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.set_frame((x, 16.0, 260.0, 26.0));
        };

        title(self.left_title, "left drop down", 20.0);
        self.left.set_frame((20, 46, 260, 40));
        self.left.set_values(["Dog", "Cat", "Sheep"]);

        title(self.right_title, "right drop down", 320.0);
        self.right.set_frame((320, 46, 260, 40));
        self.right.set_values(["Car", "Boat", "Plane"]);

        self.button.set_frame((20, 380, 260, 44));
        self.button.set_text("button").set_text_size(20);
        self.button.on_tap(move || {
            self.taps += 1;
            self.show_state();
        });

        self.state.set_text_size(18).set_multiline(true);
        self.state.set_alignment(TextAlignment::Left);
        self.state.set_frame((20, 450, 560, 130));
        self.show_state();
    }
}

impl DropDownOutsideTap {
    fn show_state(self: Weak<Self>) {
        let open = |opened: bool| if opened { "open" } else { "closed" };
        self.state.set_text(format!(
            "left {} with {}, right {} with {}, button taps {}",
            open(self.left.is_opened()),
            self.left.value(),
            open(self.right.is_opened()),
            self.right.value(),
            self.taps
        ));
    }

    /// Taps a point, then gives whether the left and the right list are open.
    fn tap(self: Weak<Self>, x: u32, y: u32) -> (bool, bool) {
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        from_main(move || {
            self.show_state();
            (self.left.is_opened(), self.right.is_opened())
        })
    }
}

impl ViewTest for DropDownOutsideTap {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(view.tap(150, 66) == (true, false), "the left list did not open");
        // a tap on the left box opened the left list
        check_colors(CHECK_1)?;

        ensure!(
            view.tap(450, 66) == (false, true),
            "a tap on the right box did not leave only the right list open"
        );
        // a tap on the right box opened the right list and closed the left one
        check_colors(CHECK_2)?;

        ensure!(
            view.tap(450, 300) == (false, false),
            "a tap on empty space left a list open"
        );
        // a tap on empty space closed the right list
        check_colors(CHECK_3)?;

        ensure!(
            view.tap(150, 66) == (true, false),
            "the left list did not open again"
        );
        ensure!(
            view.tap(150, 402) == (false, false),
            "a tap on the button left the list open"
        );
        let taps = from_main(move || view.taps);
        ensure!(taps == 1, "the button took {taps} taps");
        // a tap on the button closed the left list and the button counted 1 tap
        check_colors(CHECK_4)?;

        ensure!(
            view.tap(150, 66) == (true, false),
            "the left list did not open a third time"
        );
        // The second row, 36 points a row under a gap and a padding.
        ensure!(
            view.tap(150, 150) == (false, false),
            "the list stayed open after a pick"
        );
        let value = from_main(move || view.left.value().to_string());
        ensure!(value == "Cat", "the tap on the second row picked {value:?}");
        // a tap on the second row picked Cat
        check_colors(CHECK_5)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
     116   24 - #415b6d
     368   28 - #1d2830
     404   36 - #263540
      52   56 - #000000
     376   64 - #ffffff
     576   80 - #ffffff
      56  104 - #5ae7ff
     100  112 - #00daff
     264  112 - #00daff
      52  120 - #00daff
      72  120 - #00daff
      92  152 - #adadad
      72  176 - #e8e8e8
     104  188 - #000000
      72  192 - #e8e8e8
     120  200 - #c9c9c9
     236  208 - #3c5364
     276  208 - #3c5364
      20  212 - #476377
     196  212 - #476377
     592  256 - #597c95
     420  332 - #597c95
     124  396 - #9e9e9e
     140  400 - #3e3e3e
     128  404 - #ffffff
     176  404 - #010101
      20  412 - #ffffff
      44  516 - #090c0f
     132  516 - #435e71
     204  516 - #597c95
     348  516 - #304351
     460  516 - #597c95
";

const CHECK_2: &str = r"
     144   32 - #425b6e
     404   36 - #263540
     228   48 - #ffffff
      56   64 - #ffffff
     364   64 - #ffffff
      96   72 - #ffffff
     352  108 - #01daff
     388  112 - #5fe8ff
     564  112 - #00daff
     388  116 - #5fe8ff
     352  120 - #01daff
     364  120 - #01daff
     388  120 - #5fe8ff
     408  156 - #000000
     580  160 - #445f72
     352  184 - #010101
     396  192 - #010101
     528  208 - #3c5364
     576  208 - #3c5364
     480  212 - #476377
       4  248 - #597c95
     276  380 - #ffffff
     124  396 - #9e9e9e
     140  400 - #3e3e3e
     592  408 - #597c95
     312  512 - #395060
      72  516 - #597c95
     140  516 - #23313b
     188  516 - #597c95
     348  516 - #304351
     464  516 - #000001
     248  520 - #010101
";

const CHECK_3: &str = r"
     116   24 - #415b6d
     344   28 - #000000
     432   28 - #597c95
     148   32 - #2b3c48
     404   36 - #263540
      52   56 - #000000
      56   64 - #ffffff
     388   64 - #676767
      52   68 - #010101
      76   68 - #ffffff
     360   68 - #ffffff
      52   72 - #000000
      96   72 - #ffffff
     388   72 - #676767
     248   84 - #ffffff
     576   84 - #ffffff
       4  252 - #597c95
     592  268 - #597c95
     368  292 - #597c95
     124  396 - #9e9e9e
     140  400 - #3e3e3e
     176  404 - #010101
      20  412 - #ffffff
      44  516 - #090c0f
     140  516 - #23313b
     180  516 - #597c95
     216  516 - #597c95
     260  516 - #597c95
     320  516 - #2e414e
     384  516 - #486478
     448  516 - #394f5f
     104  520 - #000001
";

const CHECK_4: &str = r"
     116   24 - #415b6d
     344   28 - #000000
     432   28 - #597c95
     148   32 - #2b3c48
     404   36 - #263540
      52   56 - #000000
      56   64 - #ffffff
     388   64 - #676767
      52   68 - #010101
      76   68 - #ffffff
     360   68 - #ffffff
      52   72 - #000000
      96   72 - #ffffff
     388   72 - #676767
     248   84 - #ffffff
     576   84 - #ffffff
       4  252 - #597c95
     592  268 - #597c95
     368  292 - #597c95
     124  396 - #9e9e9e
     140  400 - #3e3e3e
     176  404 - #010101
      20  412 - #ffffff
      44  516 - #090c0f
     140  516 - #23313b
     180  516 - #597c95
     216  516 - #597c95
     260  516 - #597c95
     320  516 - #2e414e
     384  516 - #486478
     448  516 - #394f5f
     104  520 - #000001
";

const CHECK_5: &str = r"
     116   24 - #415b6d
      52   28 - #000001
     368   28 - #1d2830
     432   28 - #597c95
     144   32 - #425b6e
     460   32 - #344857
     404   36 - #263540
      60   64 - #ffffff
      92   64 - #adadad
     360   64 - #ffffff
     388   64 - #676767
      92   68 - #adadad
      76   72 - #ffffff
     388   72 - #676767
     248   84 - #ffffff
     576   84 - #ffffff
       4  248 - #597c95
     592  284 - #597c95
     368  292 - #597c95
     124  396 - #9e9e9e
     140  400 - #3e3e3e
     148  404 - #ffffff
     176  404 - #010101
      20  412 - #ffffff
      44  516 - #090c0f
     168  516 - #597c95
     244  516 - #597c95
     312  516 - #2c3e4a
     344  516 - #597c95
     396  516 - #25343f
     428  516 - #324553
     104  520 - #000001
";
