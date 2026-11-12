use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Container, DynamicColor, Label, Setup, TextAlignment, TextDropDown, Theme, ThemeMode, ViewData,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#F4F5F7"), Color::hex("#1A1D24"));
const INK: DynamicColor = DynamicColor::new(Color::hex("#20232A"), Color::hex("#E8EAEE"));

const CLOSED: &str = r"
    180   24 - #35383f
    284   24 - #888a90
    60   28 - #1a1d24
    68   28 - #1a1d24
    124   28 - #dadce1
    136   28 - #7f8187
    148   28 - #e4e6ea
    172   28 - #1a1d24
    180   28 - #35383f
    188   28 - #787a80
    204   28 - #dde0e4
    220   28 - #7f8187
    240   28 - #41444a
    252   28 - #1a1d24
    296   28 - #a6a8ad
    304   28 - #d1d3d8
    324   28 - #e7e9ed
    104   60 - #f2f2f4
    48   68 - #f1f1f3
    92   68 - #a1a2a6
    104   68 - #f2f2f4
    84   72 - #2a2d33
    92   72 - #a1a2a6
    80   76 - #2a2d33
    92   76 - #a1a2a6
    104   76 - #f2f2f4
    92   80 - #a1a2a6
    104   80 - #f1f1f3
    292  164 - #1a1d24
    4  252 - #1a1d24
    192  252 - #1a1d24
    392  252 - #1a1d24
";

const OPEN: &str = r"
    124   28 - #dadce1
    172   28 - #1a1d24
    240   28 - #41444a
    252   28 - #1a1d24
    296   28 - #a6a8ad
    64   32 - #74777d
    104   68 - #f2f2f4
    92   72 - #a1a2a6
    92   80 - #a1a2a6
    172  104 - #273b43
    104  108 - #00daff
    68  112 - #00d9fe
    228  116 - #273b43
    92  120 - #109ab3
    92  124 - #109ab3
    52  128 - #00daff
    92  128 - #109ab3
    104  128 - #00d9fe
    116  144 - #737579
    96  152 - #f2f2f4
    148  152 - #f2f2f4
    52  156 - #77797d
    76  160 - #2a2d33
    116  160 - #737579
    52  180 - #d3d4d6
    116  180 - #4a4d52
    56  192 - #d4d5d7
    76  192 - #5e6065
    80  196 - #575a5f
    116  200 - #4a4d52
    252  252 - #1a1d24
    392  252 - #1a1d24
";

/// A drop down the app did not style, on a page in the dark theme. Its box,
/// its text, its chevron and its open list take dark values, not the white
/// box with black text of the light theme.
#[view]
struct DropDownDark {
    #[init]
    page:    Container,
    caption: Label,
    drop:    TextDropDown,
}

impl Setup for DropDownDark {
    fn setup(self: Weak<Self>) {
        self.page.set_color(PAGE);
        self.page.place().back();

        self.caption
            .set_text("a drop down with no style of the app, dark theme")
            .set_text_size(14)
            .set_alignment(TextAlignment::Left)
            .set_text_color(INK);
        self.caption.place().t(16).lr(16).h(22);

        self.drop.set_values(vec!["Work", "Holidays", "Family"]);
        self.drop.place().t(52).l(16).size(220, 40);
    }
}

impl ViewTest for DropDownDark {
    fn canvas() -> (u32, u32) {
        (400, 260)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Dark));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        from_main(move || {
            let fill = *view.drop.color();
            ensure!(
                fill.r < 0.3 && fill.g < 0.3 && fill.b < 0.3,
                "the box is {fill:?}, not dark"
            );
            Ok(())
        })?;
        check_colors(CLOSED)?;

        // A tap on the box opens the list under it.
        inject_touches("120 72 b\n120 72 e\n390 250 m");
        wait_for_next_frame();
        check_colors(OPEN)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}
