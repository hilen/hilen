use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{ToF32, color::Color},
    refs::Weak,
    ui::{
        DrawerShell, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ViewData, ViewSubviews,
        ViewTest, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const GREEN: DynamicColor = DynamicColor::new(Color::hex("#2F9E44"), Color::hex("#51CF66"));

/// The icon of the title: 20 points wide, 6 right of the menu button, which
/// ends at 42, in the middle of the 44 points of the bar.
const ICON_LEFT: u32 = 48;
const ICON_TOP: u32 = 12;
const ICON_SIDE: u32 = 20;
/// The icon is a thin outline, a part of its pixels are green.
const GREEN_ICON: usize = 20;

/// How many pixels of the place of the icon are green. The color probes
/// skip the edges of an icon, and an outline is all edge.
fn green_pixels() -> Result<usize> {
    let shot = capture_screenshot()?;
    let mut count = 0;
    for y in 0..ICON_SIDE {
        for x in 0..ICON_SIDE {
            let pixel = shot.get_pixel(((ICON_LEFT + x).to_f32(), (ICON_TOP + y).to_f32()));
            if pixel.g > pixel.r.saturating_add(40) && pixel.g > pixel.b.saturating_add(40) {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// A narrow drawer shell, so its bar shows. The title gets a green icon left
/// of it, between the menu button and the text, and stands alone again when
/// the icon is taken away.
#[view]
struct DrawerTitleIconTest {
    status: Weak<Label>,

    #[init]
    shell: DrawerShell,
}

impl Setup for DrawerTitleIconTest {
    fn setup(mut self: Weak<Self>) {
        self.shell.place().back();
        self.shell.set_title("Website");
        self.shell.set_title_icon("tool_tag.svg", GREEN);
        self.shell.content().set_color(PAGE);

        self.status = self.shell.content().add_view::<Label>();
        self.status
            .set_text("bar: menu button, green icon, title")
            .set_text_size(16)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Center);
        self.status.place().t(40).lr(12).h(30);
    }
}

impl ViewTest for DrawerTitleIconTest {
    fn canvas() -> (u32, u32) {
        (400, 160)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        check_colors(WITH_ICON)?;
        let green = green_pixels()?;
        ensure!(
            green >= GREEN_ICON,
            "the icon of the title is green, its place has {green} green pixels"
        );

        from_main(move || {
            view.shell.clear_title_icon();
            view.status.set_text("icon taken away: menu button, title");
        });
        settle();
        check_colors(NO_ICON)?;
        let green = green_pixels()?;
        ensure!(green == 0, "the icon is gone, its place has {green} green pixels");
        checkpoint("icon taken away, the title sits next to the menu button")?;

        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

/// The green icon between the menu button and the title.
const WITH_ICON: &str = r"
             280    4 - #f9f9f9
             392    4 - #f9f9f9
             204   12 - #f9f9f9
              60   20 - #f9f9f9
             128   20 - #1d1d1f
              56   24 - #f9f9f9
             104   24 - #f9f9f9
             112   24 - #f9f9f9
             120   24 - #f9f9f9
             128   24 - #1d1d1f
             336   32 - #f9f9f9
               4   76 - #ffffff
             376   80 - #ffffff
              68   84 - #ffffff
             128  100 - #1c1c1e
             132  100 - #656566
             164  100 - #1d1d1f
             184  100 - #ffffff
             188  100 - #ffffff
             192  100 - #1c1c1e
             220  100 - #4a4a4c
             228  100 - #656566
             236  100 - #656566
             248  100 - #1c1c1e
             256  100 - #515152
             264  100 - #ffffff
             272  100 - #ffffff
             296  100 - #ffffff
             304  100 - #ffffff
               4  152 - #ffffff
              76  152 - #ffffff
             392  152 - #ffffff
";

/// The title alone right of the menu button.
const NO_ICON: &str = r"
               4    4 - #f9f9f9
             188    4 - #f9f9f9
             288    4 - #f9f9f9
             392    4 - #f9f9f9
              84   24 - #f9f9f9
             340   36 - #f9f9f9
             140   40 - #f9f9f9
             236   40 - #f9f9f9
             292   52 - #ffffff
             188   64 - #ffffff
              12   76 - #ffffff
             388   76 - #ffffff
             268   96 - #1c1c1e
             312   96 - #1c1c1e
              84  100 - #ffffff
              92  100 - #ffffff
             132  100 - #ffffff
             148  100 - #1c1c1e
             228  100 - #1c1c1e
             248  100 - #ffffff
             268  100 - #1c1c1e
             276  100 - #ffffff
             300  100 - #ffffff
             312  100 - #1c1c1e
             320  100 - #656566
              60  148 - #ffffff
               8  152 - #ffffff
             112  152 - #ffffff
             188  152 - #ffffff
             256  152 - #ffffff
             336  152 - #ffffff
             392  152 - #ffffff
";
