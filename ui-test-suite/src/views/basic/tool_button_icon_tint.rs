use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{ToF32, color::Color},
    refs::Weak,
    ui::{
        Container, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ToolButton, ViewData,
        ViewTest, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint},
};

const BAR: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
const STATUS: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const GREEN: DynamicColor = DynamicColor::new(Color::hex("#2F9E44"), Color::hex("#51CF66"));

/// Where the 2 buttons start, and where an icon starts inside a button.
const TINTED_TOP: u32 = 12;
const PLAIN_TOP: u32 = 52;
const LEFT: u32 = 12;
const ICON_LEFT: u32 = 10;
const ICON_TOP: u32 = 8;
const ICON_SIDE: u32 = 16;
/// The icon is a thin outline, about 50 of its pixels are green.
const GREEN_ICON: usize = 20;

/// How many pixels of the icon of the button at `top` are green. The color
/// probes skip the edges of an icon, and an outline is all edge.
fn green_pixels(top: u32) -> Result<usize> {
    let shot = capture_screenshot()?;
    let mut count = 0;
    for y in 0..ICON_SIDE {
        for x in 0..ICON_SIDE {
            let pixel = shot.get_pixel(((LEFT + ICON_LEFT + x).to_f32(), (top + ICON_TOP + y).to_f32()));
            if pixel.g > pixel.r.saturating_add(40) && pixel.g > pixel.b.saturating_add(40) {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// The tinted icon is green or not, and the plain one never is.
fn check_green(tinted: bool, state: &str) -> Result<()> {
    let (first, second) = (green_pixels(TINTED_TOP)?, green_pixels(PLAIN_TOP)?);
    if tinted {
        ensure!(
            first >= GREEN_ICON,
            "{state}: the tinted icon is green, it has {first} green pixels"
        );
    } else {
        ensure!(
            first == 0,
            "{state}: the first icon is not green, it has {first} green pixels"
        );
    }
    ensure!(
        second == 0,
        "{state}: the plain icon is not green, it has {second} green pixels"
    );
    Ok(())
}

/// 2 buttons with the same icon. The first has a green icon tint, the
/// second has none. The green icon stays green while its button is on, both
/// icons turn faint when disabled, and with the tint taken away the first
/// icon follows its text again.
#[view]
struct ToolButtonIconTintTest {
    #[init]
    bar:    Container,
    tinted: ToolButton,
    plain:  ToolButton,
    status: Label,
}

impl Setup for ToolButtonIconTintTest {
    fn setup(self: Weak<Self>) {
        self.status.set_alignment(TextAlignment::Center);
        self.bar.set_color(BAR);
        self.bar.place().back();

        self.tinted.set_icon("tool_tag.svg").set_text("icon with a green tint");
        self.tinted.set_icon_tint(Some(GREEN.into()));
        self.tinted.place().t(TINTED_TOP).l(LEFT);

        self.plain.set_icon("tool_tag.svg").set_text("icon with no tint");
        self.plain.place().t(PLAIN_TOP).l(LEFT);

        self.status.set_text_size(16).set_text_color(STATUS);
        self.status.place().t(100).lr(12).h(30);
        self.status.set_text("at rest: green icon, dark icon");
    }
}

impl ViewTest for ToolButtonIconTintTest {
    fn canvas() -> (u32, u32) {
        (400, 150)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(REST)?;
        check_green(true, "at rest")?;

        from_main(move || {
            view.tinted.set_on(true);
            view.plain.set_on(true);
            view.status.set_text("both on: green icon, blue icon");
        });
        wait_for_next_frame();
        check_colors(ON)?;
        check_green(true, "both on")?;

        from_main(move || {
            view.tinted.set_on(false).set_enabled(false);
            view.plain.set_on(false).set_enabled(false);
            view.status.set_text("both disabled: both icons faint");
        });
        wait_for_next_frame();
        check_colors(DISABLED)?;
        check_green(false, "both disabled")?;

        from_main(move || {
            view.tinted.set_enabled(true).set_icon_tint(None);
            view.plain.set_enabled(true);
            view.status.set_text("tint taken away: both icons dark");
        });
        wait_for_next_frame();
        check_colors(NO_TINT)?;
        check_green(false, "tint taken away")?;
        checkpoint("tint taken away, both icons dark")?;

        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

/// At rest: the first icon green, its text and the second button dark.
const REST: &str = r"
             240    4 - #f4f4f6
             292    4 - #f4f4f6
             392    4 - #f4f4f6
              28   28 - #f4f4f6
              52   28 - #f4f4f6
             120   28 - #1c1c1e
             156   28 - #f4f4f6
             316   48 - #f4f4f6
             208   56 - #f4f4f6
             264   60 - #f4f4f6
              28   68 - #f4f4f6
              52   68 - #f4f4f6
             124   68 - #4b4b4d
             132   68 - #1c1c1e
             140   68 - #1c1c1e
             380   76 - #f4f4f6
             128  116 - #616163
             144  116 - #f4f4f6
             156  116 - #f4f4f6
             160  116 - #1c1c1e
             172  116 - #616163
             180  116 - #616163
             204  116 - #f4f4f6
             208  116 - #f4f4f6
             212  116 - #f4f4f6
             220  116 - #1c1c1e
             260  116 - #1c1c1e
             280  116 - #f4f4f6
              68  140 - #f4f4f6
               4  144 - #f4f4f6
             332  144 - #f4f4f6
             392  144 - #f4f4f6
";

/// Both on: the first icon still green, its text and the second button blue.
const ON: &str = r"
             248    4 - #f4f4f6
             392    4 - #f4f4f6
              32   28 - #e1ecff
             120   28 - #0a6cff
             156   28 - #e1ecff
             320   44 - #f4f4f6
             216   56 - #f4f4f6
             268   64 - #f4f4f6
              28   68 - #e1ecff
             124   68 - #3888ff
             132   68 - #0a6cff
             140   68 - #0a6cff
             392   80 - #f4f4f6
             332   96 - #f4f4f6
             116  112 - #1c1c1e
              48  116 - #f4f4f6
             100  116 - #f4f4f6
             108  116 - #f4f4f6
             116  116 - #1c1c1e
             136  116 - #f4f4f6
             148  116 - #1c1c1e
             168  116 - #48484a
             176  116 - #616163
             184  116 - #616163
             196  116 - #1c1c1e
             204  116 - #4e4e50
             212  116 - #f4f4f6
             220  116 - #f4f4f6
             244  116 - #f4f4f6
             284  116 - #f4f4f6
               4  144 - #f4f4f6
             356  144 - #f4f4f6
";

/// Both disabled: every icon and text faint.
const DISABLED: &str = r"
             248    4 - #f4f4f6
             392    4 - #f4f4f6
              32   28 - #f4f4f6
             120   28 - #b4b4bc
             156   28 - #f4f4f6
             320   36 - #f4f4f6
              76   52 - #f4f4f6
             212   56 - #f4f4f6
             268   60 - #f4f4f6
              28   68 - #f4f4f6
             116   68 - #f4f4f6
             124   68 - #c2c2c9
             132   68 - #b4b4bc
             140   68 - #b4b4bc
             380   76 - #f4f4f6
             116  112 - #dfdfe1
             160  112 - #1c1c1e
             304  112 - #1c1c1e
              96  116 - #f4f4f6
             112  116 - #4b4b4d
             132  116 - #f4f4f6
             156  116 - #1c1c1e
             160  116 - #1c1c1e
             176  116 - #616163
             184  116 - #f4f4f6
             200  116 - #f4f4f6
             240  116 - #f4f4f6
             248  116 - #f4f4f6
             300  116 - #1c1c1e
             304  116 - #1c1c1e
               4  144 - #f4f4f6
             392  144 - #f4f4f6
";

/// The tint taken away: both icons dark like their texts.
const NO_TINT: &str = r"
             200    4 - #f4f4f6
             252    4 - #f4f4f6
             392    4 - #f4f4f6
              28   28 - #f4f4f6
              52   28 - #f4f4f6
             120   28 - #1c1c1e
             156   28 - #f4f4f6
             320   40 - #f4f4f6
             220   56 - #f4f4f6
             272   60 - #f4f4f6
              28   68 - #f4f4f6
              52   68 - #f4f4f6
             124   68 - #4b4b4d
             132   68 - #1c1c1e
             140   68 - #1c1c1e
             380   76 - #f4f4f6
             204  112 - #1c1c1e
             288  112 - #1c1c1e
              92  116 - #f4f4f6
             140  116 - #616163
             152  116 - #3d3d3f
             164  116 - #1c1c1e
             204  116 - #1c1c1e
             208  116 - #f4f4f6
             216  116 - #f4f4f6
             224  116 - #f4f4f6
             240  116 - #1c1c1e
             252  116 - #1c1c1e
             284  116 - #f4f4f6
             300  116 - #1d1d1f
               4  144 - #f4f4f6
             392  144 - #f4f4f6
";
