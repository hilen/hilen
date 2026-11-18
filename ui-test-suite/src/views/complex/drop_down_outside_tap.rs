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
      432   28 - #597c95
      564   48 - #ffffff
       36   56 - #000000
       80   64 - #ffffff
      360   64 - #ffffff
       36   72 - #000000
      464   84 - #ffffff
       44  104 - #01daff
       36  108 - #01daff
       80  108 - #00daff
      264  112 - #00daff
       36  120 - #00daff
       56  120 - #00daff
       84  128 - #01daff
       44  180 - #ffffff
       88  188 - #000000
      220  208 - #3c5364
       20  212 - #476377
      152  212 - #476377
      592  248 - #597c95
      396  316 - #597c95
       24  380 - #ffffff
      128  404 - #ffffff
      176  404 - #010101
      592  416 - #597c95
       44  516 - #090c0f
      276  516 - #597c95
      364  516 - #597c95
      460  516 - #597c95
      152  520 - #000000
      204  520 - #000001
      592  592 - #597c95
";

const CHECK_2: &str = r"
      112   28 - #597c95
      432   28 - #597c95
      192   48 - #ffffff
       40   64 - #ffffff
      348   64 - #ffffff
       80   72 - #ffffff
      336  108 - #01daff
      360  108 - #00daff
      564  112 - #00daff
      476  116 - #ebfcff
      336  120 - #01daff
      348  120 - #01daff
      356  120 - #01daff
      392  144 - #000000
      336  152 - #000000
      336  184 - #010101
      380  192 - #010101
      524  208 - #3c5364
      472  212 - #476377
      580  212 - #476377
      172  232 - #597c95
        4  264 - #597c95
      592  380 - #597c95
       20  392 - #ffffff
      148  404 - #ffffff
      276  408 - #ffffff
      144  512 - #000000
       72  516 - #597c95
      216  516 - #597c95
      440  516 - #000001
      332  520 - #000000
      592  592 - #597c95
";

const CHECK_3: &str = r"
      112   28 - #597c95
      432   28 - #597c95
      384   32 - #000000
       36   56 - #000000
       36   60 - #010101
       36   64 - #010101
       40   64 - #ffffff
       36   68 - #010101
       60   68 - #ffffff
      344   68 - #ffffff
       36   72 - #000000
       80   72 - #ffffff
      240   84 - #ffffff
      492   84 - #ffffff
      576   84 - #ffffff
      148  232 - #597c95
      592  284 - #597c95
      368  292 - #597c95
      276  380 - #ffffff
       20  384 - #ffffff
      128  404 - #ffffff
      176  404 - #010101
      588  436 - #597c95
      164  512 - #000000
       72  516 - #597c95
      112  516 - #597c95
      216  516 - #597c95
      316  516 - #000000
      396  516 - #050708
      456  516 - #597c95
      248  520 - #010101
      592  592 - #597c95
";

const CHECK_4: &str = r"
      112   28 - #597c95
      432   28 - #597c95
      384   32 - #000000
       36   56 - #000000
       36   60 - #010101
       36   64 - #010101
       40   64 - #ffffff
       36   68 - #010101
       60   68 - #ffffff
      344   68 - #ffffff
       36   72 - #000000
       80   72 - #ffffff
      240   84 - #ffffff
      492   84 - #ffffff
      576   84 - #ffffff
      148  232 - #597c95
      592  284 - #597c95
      368  292 - #597c95
      276  380 - #ffffff
       20  384 - #ffffff
      128  404 - #ffffff
      176  404 - #010101
      588  436 - #597c95
      164  512 - #000000
       72  516 - #597c95
      112  516 - #597c95
      216  516 - #597c95
      316  516 - #000000
      396  516 - #050708
      456  516 - #597c95
      248  520 - #010101
      592  592 - #597c95
";

const CHECK_5: &str = r"
      112   28 - #597c95
      344   28 - #000000
      432   28 - #597c95
      384   32 - #000000
       44   64 - #ffffff
      344   68 - #ffffff
       60   72 - #ffffff
      360   72 - #ffffff
      164   84 - #ffffff
      240   84 - #ffffff
      492   84 - #ffffff
      576   84 - #ffffff
      480  216 - #597c95
      128  224 - #597c95
      592  288 - #597c95
      368  292 - #597c95
      276  380 - #ffffff
       20  384 - #ffffff
      128  404 - #ffffff
      148  404 - #ffffff
      176  404 - #010101
      580  440 - #597c95
      144  512 - #000000
      336  512 - #000000
       72  516 - #597c95
      112  516 - #597c95
      244  516 - #597c95
      284  516 - #597c95
      380  516 - #010101
      444  516 - #597c95
      212  520 - #000000
      592  592 - #597c95
";
