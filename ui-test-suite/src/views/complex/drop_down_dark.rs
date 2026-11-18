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
     132   24 - #e8eaee
     180   24 - #35383f
      44   28 - #e8eaee
      60   28 - #1a1d24
      68   28 - #1a1d24
     148   28 - #e4e6ea
     172   28 - #1a1d24
     204   28 - #dde0e4
     244   28 - #1a1d24
     252   28 - #1a1d24
     296   28 - #a6a8ad
     304   28 - #d1d3d8
     324   28 - #e7e9ed
      88   60 - #f2f2f4
      88   64 - #f2f2f4
      32   68 - #f1f1f3
      88   68 - #f2f2f4
      64   72 - #2a2d33
      68   72 - #2a2d33
      88   72 - #f2f2f4
      64   76 - #2a2d33
      68   76 - #2a2d33
      88   76 - #f2f2f4
      48   80 - #f2f2f4
      88   80 - #f1f1f3
     392  124 - #1a1d24
     176  144 - #1a1d24
     284  164 - #1a1d24
      84  184 - #1a1d24
       4  244 - #1a1d24
     180  252 - #1a1d24
     392  252 - #1a1d24
";

const OPEN: &str = r"
     180   24 - #35383f
      68   28 - #1a1d24
     124   28 - #dadce1
     172   28 - #1a1d24
     244   28 - #1a1d24
     252   28 - #1a1d24
     296   28 - #a6a8ad
     304   28 - #d1d3d8
      32   68 - #f1f1f3
      68   76 - #2a2d33
     168  104 - #273b43
      88  108 - #00daff
      52  112 - #00d9fe
     128  112 - #273b43
      32  116 - #00d9fe
     228  116 - #273b43
      88  120 - #00daff
     392  124 - #1a1d24
      36  128 - #00daff
      48  128 - #00daff
      88  128 - #00d9fe
     192  136 - #273b43
     132  152 - #f2f2f4
      96  156 - #2a2d33
      72  160 - #f2f2f4
      32  164 - #f2f2f4
     308  172 - #1a1d24
      76  192 - #f2f2f4
     108  196 - #2a2d33
      32  200 - #f2f2f4
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
