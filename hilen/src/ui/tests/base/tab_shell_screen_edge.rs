use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{LossyConvert, color::Color},
    ui::{
        DynamicColor, Label, Setup, Tab, TabColors, TabShell, TextAlignment, U8Color, UIManager, ViewData,
        ViewTest, view,
    },
    ui_test::{capture_screenshot, checkpoint},
};

const PAGE: Color = Color::hex("#FFFFFF");
const BAR: Color = Color::hex("#2060C0");

/// The bar of a `TabShell` paints down to the bottom edge of the screen, past
/// the home bar of a phone, so no strip of the clear color stays under it.
/// The app views end above the home bar, and the bar once ended with them.
/// The test drops the canvas, only the real screen has a home bar.
#[view]
struct TabShellScreenEdge {
    #[init]
    shell: TabShell,
}

impl Setup for TabShellScreenEdge {
    fn setup(self: Weak<Self>) {
        self.shell.place().back();
        let plain = |color| DynamicColor::new(color, color);
        self.shell.set_colors(TabColors {
            bar:      plain(BAR),
            line:     plain(BAR),
            selected: plain(PAGE),
            normal:   plain(PAGE),
        });
        self.shell.set_tabs(
            vec![
                Tab::new("Lists", "tab_lists.svg"),
                Tab::new("Friends", "tab_friends.svg"),
            ],
            |_| {
                let page = Label::new();
                page.set_text("page").set_alignment(TextAlignment::Center).set_color(PAGE);
                page
            },
        );
        self.shell.select(0);
    }
}

impl ViewTest for TabShellScreenEdge {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        from_main(|| UIManager::root_view().clear_test_canvas());
        wait_for_next_frame();
        wait_for_next_frame();

        let shot = capture_screenshot()?;
        let (width, height) = (shot.size.width, shot.size.height);
        let inset = from_main(|| UIManager::system_insets().bottom * UIManager::scale());
        let x: f32 = (width / 2).lossy_convert();
        let bottom: f32 = height.saturating_sub(2).lossy_convert();
        // In the middle of the home bar strip, at the bottom row where there is
        // none.
        let strip = bottom - inset / 2.0;

        for (y, what) in [
            (bottom, "the bottom row of the screen"),
            (strip, "the home bar strip"),
        ] {
            let seen = shot.get_pixel((x, y));
            ensure!(
                seen.diff_u8(U8Color::from(BAR)) < 12,
                "{what} at {x} {y} is {}, not the bar {}, the screen is {width} by {height} with a \
                 bottom inset of {inset} pixels",
                seen.as_hex(),
                U8Color::from(BAR).as_hex()
            );
        }
        checkpoint("whole screen: the bar reaches the bottom edge of the screen")
    }
}
