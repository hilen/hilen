use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Color, Label, RowIcon, Setup, TextAlignment, TextDropDown, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// An icon the app built in, drawn in black.
const BYTES: &[u8] = include_bytes!("../basic/tool_button_bytes.svg");

const CLOSED: &str = r"
             164   40 - #304351
              44   44 - #2f424f
              72   44 - #597c95
             104   44 - #597c95
             132   44 - #10171b
             164   44 - #304351
             188   44 - #425c6e
             196   44 - #466175
             204   44 - #090c0f
             224   44 - #000001
             256   44 - #090c0f
             284   44 - #000000
             356   44 - #597c95
             320   48 - #010101
             216   80 - #000000
             288   80 - #ffffff
             272   84 - #d5d5d5
             140   88 - #ffffff
             232   92 - #ffffff
             164   96 - #ffffff
             200   96 - #9b9b9b
             252   96 - #000000
              80  100 - #ccd6e6
             136  100 - #000000
             200  100 - #9b9b9b
             216  100 - #000000
             272  100 - #d5d5d5
             316  104 - #ffffff
             112  112 - #ffffff
             300  436 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";
const OPENED: &str = r"
             164   40 - #304351
              44   44 - #2f424f
              96   44 - #597c95
             188   44 - #425c6e
             196   44 - #466175
             204   44 - #090c0f
             252   44 - #000000
             300   44 - #090c0f
             356   44 - #597c95
              40   48 - #000000
             272   80 - #d5d5d5
             136   88 - #010101
             232   92 - #ffffff
             200   96 - #9b9b9b
              80  100 - #d9591a
             200  100 - #9b9b9b
             216  132 - #d9591a
             304  140 - #d9591a
             148  144 - #d95a1b
             200  144 - #eeb69a
              80  148 - #ccd6e6
             200  148 - #eeb69a
             264  148 - #fcf2ed
             136  172 - #000000
             176  176 - #bcbcbc
             176  180 - #bcbcbc
              84  192 - #ffffff
             232  196 - #ffffff
             132  204 - #476377
             312  204 - #486478
             176  592 - #597c95
             592  592 - #597c95
";
const PICKED: &str = r"
             164   40 - #304351
              44   44 - #2f424f
             104   44 - #597c95
             132   44 - #10171b
             160   44 - #0e1317
             164   44 - #304351
             188   44 - #425c6e
             196   44 - #466175
             224   44 - #000001
             252   44 - #000000
             288   44 - #090c0f
             320   44 - #597c95
             356   44 - #597c95
              40   48 - #000000
              80   84 - #ccd6e6
             136   84 - #000000
             176   84 - #bcbcbc
             152   88 - #000000
             232   88 - #ffffff
             168   92 - #ffffff
             176   92 - #bcbcbc
             316   92 - #ffffff
              80   96 - #ccd6e6
             164   96 - #ffffff
             176   96 - #bcbcbc
             188   96 - #ffffff
             140  100 - #727272
             144  100 - #727272
             272  112 - #ffffff
             300  440 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";

/// A drop down of texts with an icon before each one: an icon from a file of
/// `assets/images` and one from bytes. The icons show in the closed box and in
/// the open list, in the color of the text of their row.
#[view]
struct TextDropDownIcons {
    #[init]
    title: Label,
    drop:  TextDropDown,
}

impl Setup for TextDropDownIcons {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text_size(18).set_alignment(TextAlignment::Left);
        self.title.set_text("a drop down with an icon before each text");
        self.title.place().t(30).lr(20).h(26);

        self.drop.set_values(["Download", "Lists"]);
        self.drop.set_icons([
            RowIcon::file("tool_download.svg"),
            RowIcon::data(BYTES, "text_drop_down_lists.svg"),
        ]);
        self.drop.set_accent_color(Color::rgb(0.85, 0.35, 0.10));
        self.drop
            .set_border_width(1)
            .set_border_color(Color::rgb(0.80, 0.84, 0.90))
            .set_corner_radius(10);
        self.drop.place().t(70).l(80).size(240, 44);
    }
}

impl ViewTest for TextDropDownIcons {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("500 500 m");
        wait_for_next_frame();
        check_colors(CLOSED)?;

        // The box, it opens the list under itself.
        inject_touches("200 92 b\n200 92 e\n500 500 m");
        wait_for_next_frame();
        check_colors(OPENED)?;

        // The panel opens 6 under the box at y 120, its rows start 4 lower and
        // are 36 tall, Lists is 160..196.
        inject_touches("200 178 b\n200 178 e\n500 500 m");
        wait_for_next_frame();
        from_main(move || {
            ensure!(view.drop.value() == "Lists", "picked {}", view.drop.value());
            Ok(())
        })?;
        check_colors(PICKED)?;
        Ok(())
    }
}
