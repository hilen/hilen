use anyhow::Result;
use test_engine::{
    AppRunner,
    gm::color::{Color, WHITE},
    refs::Weak,
    ui::{Container, Setup, ViewData, ViewFrame, ViewTest, view},
};

/// The engine promise this test pins: a color written as a CSS hex
/// string lands on screen as exactly that hex, and alpha blending uses
/// the same math CSS uses, blend the encoded bytes directly. Every
/// expectation below is hand computed from the CSS rules, never
/// recorded, so it can not inherit a pipeline bug.
#[view]
struct CssColors {
    #[init]
    green: Container,
    dark:  Container,
    muted: Container,
    tint:  Container,
    mid:   Container,
}

impl Setup for CssColors {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        self.green.set_color(Color::hex("#22c55e"));
        self.green.set_frame((20, 20, 100, 100));

        self.dark.set_color(Color::hex("#0a0a0a"));
        self.dark.set_frame((140, 20, 100, 100));

        // rgba(10, 10, 10, 0.55) over white, the CSS result is #787878.
        self.muted.set_color(Color::hex("#0a0a0a").with_alpha(0.55));
        self.muted.set_frame((260, 20, 100, 100));

        // rgba(34, 197, 94, 0.14) over white, the CSS result is #e0f7e8.
        self.tint.set_color(Color::hex("#22c55e").with_alpha(0.14));
        self.tint.set_frame((20, 140, 100, 100));

        // Raw fractions mean the same bytes, 0.5 is #808080, never a
        // linear value the pipeline brightens to #bbbbbb.
        self.mid.set_color(Color::rgb(0.5, 0.5, 0.5));
        self.mid.set_frame((140, 140, 100, 100));
    }
}

fn check_exact(pos: (i32, i32), expected: &str) -> Result<()> {
    let screenshot = AppRunner::take_screenshot()?;
    let pixel = screenshot.get_pixel(pos).as_hex();

    if pixel != expected {
        anyhow::bail!("css color mismatch at {pos:?}: expected {expected}, got {pixel}");
    }

    Ok(())
}

impl ViewTest for CssColors {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_exact((70, 70), "#22c55e")?;
        check_exact((190, 70), "#0a0a0a")?;
        check_exact((310, 70), "#787878")?;
        check_exact((70, 190), "#e0f7e8")?;
        check_exact((190, 190), "#808080")?;
        check_exact((500, 300), "#ffffff")?;

        Ok(())
    }
}
