use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        DynamicColor, Label, Setup, TextAlignment, TextDropDown, Theme, ThemeMode, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// A drop down whose accent is a light and dark pair: the open border, the
/// picked row and its check mark take the color of the theme shown.
#[view]
struct DropDownAccentTest {
    #[init]
    status:  Label,
    caption: Label,
    drop:    TextDropDown,
}

impl Setup for DropDownAccentTest {
    fn setup(self: Weak<Self>) {
        self.set_color(pair("#F4F5F7", "#1A1D24"));
        self.status
            .set_text("open, the accent of the light theme")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status.set_text_color(pair("#20232A", "#E8EAEE"));
        self.status.place().t(10).lr(10).h(30);
        self.caption
            .set_text("drop down with an orange accent pair")
            .set_text_size(14)
            .set_alignment(TextAlignment::Left);
        self.caption.set_text_color(pair("#55596A", "#9AA0B4"));
        self.caption.place().t(50).lr(20).h(24);

        self.drop.set_values(["Admin", "Member", "Viewer"]);
        self.drop.set_value("Member");
        self.drop.set_color(pair("#FFFFFF", "#252A35"));
        self.drop
            .set_border_color(pair("#D0D4DC", "#3A4150"))
            .set_border_width(1)
            .set_corner_radius(8);
        self.drop.set_text_color(pair("#20232A", "#E8EAEE"));
        self.drop.set_accent_color(pair("#D9480F", "#FFA94D"));
        self.drop.place().t(90).l(20).size(220, 40);
    }
}

impl ViewTest for DropDownAccentTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        toggle();
        from_main(move || {
            ensure!(view.drop.is_opened(), "the tap opened the list");
            Ok(())
        })?;
        check_colors(OPEN_LIGHT)?;
        checkpoint("open in light, the border, Member and its check mark in dark orange")?;

        toggle();
        from_main(move || {
            ensure!(!view.drop.is_opened(), "the second tap closed the list");
            Theme::set_mode(ThemeMode::Dark);
            view.status.set_text("open again, the accent of the dark theme");
            Ok(())
        })?;
        settle();
        toggle();
        check_colors(OPEN_DARK)?;
        checkpoint("open in dark, the border, Member and its check mark in light orange")?;

        toggle();
        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// A tap on the box, then the pointer away from it, so no hover wash lies
/// over the box.
fn toggle() {
    inject_touches("130 110 b\n130 110 e\n500 500 m");
    settle();
}

fn pair(light: &str, dark: &str) -> DynamicColor {
    DynamicColor::new(Color::hex(light), Color::hex(dark))
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

const OPEN_LIGHT: &str = r"
             260   20 - #3d4046
             108   24 - #f4f5f7
             272   24 - #f4f5f7
              52   28 - #f4f5f7
             196   28 - #32353c
             260   28 - #3d4046
             124   60 - #dfe0e4
             100   64 - #585c6d
             148   64 - #777a88
             172   64 - #55596a
             180   64 - #787b89
             236   64 - #585c6d
              56  112 - #21242b
             108  112 - #20232a
             184  136 - #d0d4dc
             240  144 - #bebec0
              80  152 - #21242b
              16  156 - #d1d2d3
             120  156 - #20232a
             100  192 - #d9480f
             224  192 - #d9480f
             120  196 - #d9480f
             144  196 - #d94911
             592  196 - #f4f5f7
              56  200 - #d94910
             112  232 - #20232a
             240  248 - #bcbdbe
              28  252 - #a4a5a6
             176  252 - #a4a5a6
              76  260 - #dedfe1
             140  592 - #f4f5f7
             592  592 - #f4f5f7
";

const OPEN_DARK: &str = r"
             308   20 - #cbcdd1
             124   24 - #cfd1d5
             276   24 - #1a1d24
              52   28 - #1a1d24
             136   28 - #d7d9dd
             340   28 - #e0e2e6
             100   64 - #979db1
             148   64 - #7f8496
             172   64 - #9aa0b4
             180   64 - #7e8394
             236   64 - #979db1
              20  100 - #ffa94d
              64  112 - #e8eaee
             108  116 - #e8eaee
             204  136 - #d0d4dc
             120  156 - #e8eaee
             156  176 - #363437
              24  180 - #323236
              96  188 - #fea94d
              68  192 - #363437
             224  192 - #ffa94d
             120  196 - #ffa94d
              36  200 - #fea94d
             112  200 - #363437
             132  200 - #363437
             184  208 - #363437
             112  232 - #e8eaee
              68  236 - #252a35
              20  240 - #d0d4dc
             592  252 - #1a1d24
             216  592 - #1a1d24
             592  592 - #1a1d24
";
