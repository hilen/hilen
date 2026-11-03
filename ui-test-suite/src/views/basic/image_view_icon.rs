use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Container, DynamicColor, ImageView, Label, Setup, TextAlignment, Theme, ThemeMode, ViewData,
        ViewTest, view,
    },
    ui_test::check_colors,
};

const PANEL: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const ICON: DynamicColor = DynamicColor::new(Color::hex("#0A6CFF"), Color::hex("#FFB020"));
const FIXED: Color = Color::hex("#D02020");

/// 2 icons from black SVGs. The first takes a theme pair, blue in the light
/// theme and orange in the dark one. The second takes a plain red that stays
/// in both themes. The labels name each one.
#[view]
struct ImageViewIcon {
    #[init]
    panel:       Container,
    theme_icon:  ImageView,
    theme_label: Label,
    fixed_icon:  ImageView,
    fixed_label: Label,
}

impl Setup for ImageViewIcon {
    fn setup(self: Weak<Self>) {
        self.panel.set_color(PANEL);
        self.panel.place().back();

        self.theme_icon.set_icon("tool_save.svg", ICON);
        self.theme_icon.place().t(16).l(16).size(48, 48);
        self.fixed_icon.set_icon("tool_tag.svg", FIXED);
        self.fixed_icon.place().t(80).l(16).size(48, 48);

        for (label, text, icon) in [
            (
                self.theme_label,
                "theme icon: blue in light, orange in dark",
                self.theme_icon,
            ),
            (
                self.fixed_label,
                "fixed icon: red in both themes",
                self.fixed_icon,
            ),
        ] {
            label
                .set_text(text)
                .set_text_size(16)
                .set_text_color(TEXT)
                .set_alignment(TextAlignment::Left);
            label.place().same_y(icon).l(80).r(16).h(48);
        }
    }
}

impl ViewTest for ImageViewIcon {
    fn canvas() -> (u32, u32) {
        (440, 144)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(LIGHT)?;

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        wait_for_next_frame();
        check_colors(DARK)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// Light theme: the theme icon blue, the fixed icon red.
const LIGHT: &str = r"
             432    4 - #f4f4f6
              36   32 - #0a6cff
              40   32 - #0a6cff
              44   32 - #0a6cff
             252   36 - #1c1c1e
             100   40 - #f4f4f6
             120   40 - #1c1c1e
             136   40 - #88888a
             160   40 - #f4f4f6
             188   40 - #f4f4f6
             240   40 - #8e8e90
             296   40 - #1c1c1e
             316   40 - #88888a
             348   40 - #1c1c1e
             360   40 - #9e9ea0
             420   72 - #f4f4f6
              24   84 - #d02020
              52   96 - #d02020
             216  100 - #b2b2b4
             240  100 - #323234
             116  104 - #88888a
             136  104 - #4e4e50
             156  104 - #9e9ea0
             200  104 - #363638
             216  104 - #b2b2b4
             220  104 - #f4f4f6
             236  104 - #3a3a3c
             292  104 - #88888a
              24  108 - #d02020
             364  112 - #f4f4f6
              36  120 - #d02020
             432  136 - #f4f4f6
";

/// Dark theme: the theme icon orange, the fixed icon still red.
const DARK: &str = r"
             432    4 - #27272a
              36   32 - #ffb020
              40   32 - #ffb020
              44   32 - #ffb020
             184   36 - #cacacc
             100   40 - #27272a
             120   40 - #f2f2f4
             136   40 - #8c8c8f
             160   40 - #27272a
             248   40 - #f2f2f4
             276   40 - #27272a
             296   40 - #f2f2f4
             316   40 - #8c8c8f
             348   40 - #f2f2f4
             360   40 - #77777a
             420   72 - #27272a
              24   84 - #d02020
              52   96 - #d02020
             216  100 - #656568
             240  100 - #dddddf
             116  104 - #8c8c8f
             136  104 - #c3c3c5
             156  104 - #77777a
             200  104 - #d9d9db
             216  104 - #656568
             220  104 - #27272a
             256  104 - #27272a
             292  104 - #8c8c8f
              24  108 - #d02020
             364  112 - #27272a
              36  120 - #d02020
             432  136 - #27272a
";
