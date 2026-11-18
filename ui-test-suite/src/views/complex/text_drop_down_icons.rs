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
               72   44 - #597c95
               96   44 - #597c95
              132   44 - #10171b
              160   44 - #0e1317
              204   44 - #090c0f
              220   44 - #597c95
              256   44 - #090c0f
              272   44 - #597c95
              288   44 - #090c0f
              356   44 - #597c95
               40   48 - #000000
              320   48 - #010101
              200   80 - #000000
              312   80 - #ffffff
              276   84 - #ffffff
              124   88 - #ffffff
              176   88 - #010101
              192   88 - #000000
              216   92 - #ffffff
              252   92 - #ffffff
              148   96 - #ffffff
              236   96 - #000000
              248   96 - #ffffff
              120  100 - #000000
              172  100 - #000000
              200  100 - #000000
               84  108 - #ffffff
              308  112 - #ffffff
              592  268 - #597c95
              296  440 - #597c95
                4  592 - #597c95
              592  592 - #597c95
";
const OPENED: &str = r"
               96   44 - #597c95
              132   44 - #10171b
              160   44 - #0e1317
              204   44 - #090c0f
              244   44 - #597c95
              284   44 - #000000
              356   44 - #597c95
               40   48 - #000000
              320   48 - #010101
              120   84 - #010101
              316   88 - #ffffff
              200   92 - #000000
              252   92 - #ffffff
              160   96 - #010101
               80  100 - #d9591a
              120  132 - #d9591a
              192  140 - #d95a1b
              304  140 - #d9591a
              132  144 - #d95a1b
              220  144 - #d9591a
              248  148 - #fcf2ed
              120  172 - #000000
              176  176 - #ffffff
              148  184 - #ffffff
              316  192 - #ffffff
               88  196 - #ffffff
              224  200 - #3c5364
              188  204 - #476377
              260  204 - #476377
              300  508 - #597c95
                4  592 - #597c95
              592  592 - #597c95
";
const PICKED: &str = r"
               56   44 - #597c95
               72   44 - #597c95
               96   44 - #597c95
              132   44 - #10171b
              160   44 - #0e1317
              204   44 - #090c0f
              224   44 - #000001
              244   44 - #597c95
              284   44 - #000000
              300   44 - #090c0f
              320   44 - #597c95
              352   44 - #090c0f
               40   48 - #000000
              120   84 - #000000
              216   84 - #ffffff
              276   84 - #ffffff
              136   88 - #000000
              172   88 - #000000
              152   92 - #ffffff
              144   96 - #ffffff
              148   96 - #ffffff
              172   96 - #ffffff
              120  100 - #000000
              136  100 - #000000
              180  100 - #010101
              312  108 - #ffffff
              244  112 - #ffffff
              592  268 - #597c95
                4  332 - #597c95
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
    fn setup(self: Weak<Self>) {
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
