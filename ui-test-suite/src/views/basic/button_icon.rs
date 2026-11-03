use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Button, Container, DynamicColor, Label, Setup, Theme, ThemeMode, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

const PANEL: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
const FILL: DynamicColor = DynamicColor::new(Color::hex("#E2E2E8"), Color::hex("#3A3A40"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const ACCENT: Color = Color::hex("#0A6CFF");

/// Filled buttons with an icon left of the text, as a side panel and a top
/// bar have them: a wide one across the panel, a blue one as wide as its
/// owner made it, one with only an icon and a disabled one. The icon and
/// the text sit as 1 block in the middle, the fill stays the button's own.
/// The status line names what the taps did.
#[view]
struct ButtonIcon {
    taps: u32,

    #[init]
    panel:    Container,
    wide:     Button,
    open:     Button,
    only:     Button,
    disabled: Button,
    status:   Label,
}

impl ButtonIcon {
    fn show_status(&self) {
        self.status.set_text(format!("buttons: wide tapped {} times", self.taps));
    }
}

fn style(button: Weak<Button>, icon: &str, text: &str) {
    button
        .set_text(text)
        .set_text_size(14)
        .set_text_color(TEXT)
        .set_color(FILL)
        .set_corner_radius(8);
    button.set_icon(icon);
}

impl Setup for ButtonIcon {
    fn setup(mut self: Weak<Self>) {
        self.panel.set_color(PANEL);
        self.panel.place().back();

        style(self.wide, "tool_save.svg", "Save the project");
        self.wide.place().t(16).lr(16).h(36);
        style(self.open, "tool_download.svg", "Open");
        self.open.set_color(ACCENT).set_text_color(Color::hex("#FFFFFF"));
        self.open.place().t(68).l(16).size(140, 36);
        style(self.only, "tool_tag.svg", "");
        self.only.place().t(68).l(172).size(36, 36);
        style(self.disabled, "tool_download.svg", "Export");
        self.disabled.place().t(68).l(224).size(140, 36);
        self.disabled.set_enabled(false);
        self.disabled.on_tap(move || self.taps += 100);

        self.wide.on_tap(move || {
            self.taps += 1;
            self.show_status();
        });

        self.status.set_text_size(16).set_text_color(TEXT);
        self.status.place().t(120).lr(16).h(30);
        self.show_status();
    }
}

impl ViewTest for ButtonIcon {
    fn canvas() -> (u32, u32) {
        (600, 170)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(LIGHT)?;

        tap(view.wide);
        tap(view.disabled);
        from_main(move || assert_eq!(view.taps, 1));

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        wait_for_next_frame();
        check_colors(DARK)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// A tap near the right end of the button, away from the icon and the text,
/// so the marker of human mode covers no probe.
fn tap(button: Weak<Button>) {
    let (x, y) = from_main(move || (button.max_x() - 6.0, button.y() + 6.0));
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
}

/// Light theme: the icons and the texts in the middle of their buttons.
const LIGHT: &str = r"
             464    4 - #f4f4f6
             592    4 - #f4f4f6
             288   36 - #e2e2e8
             300   36 - #29292b
             312   36 - #e0e0e6
             340   36 - #86868a
              48   68 - #0a6cff
             356   68 - #d8d8da
             152   72 - #0a6cff
             228   72 - #d6d6d8
              84   84 - #0a6cff
             104   88 - #0d6eff
             112   88 - #67a4ff
             300   88 - #bdbdc0
             316   88 - #b0b0b3
              16   92 - #0a6cff
             288   92 - #d6d6d8
             480   96 - #f4f4f6
             252  100 - #d6d6d8
             344  132 - #202022
             200  136 - #f4f4f6
             216  136 - #8c8c8e
             240  136 - #9e9ea0
             288  136 - #616163
             308  136 - #1c1c1e
             320  136 - #b2b2b4
             332  136 - #616163
             364  136 - #a2a2a4
             384  136 - #1c1c1e
             392  136 - #616163
             320  140 - #b2b2b4
             548  164 - #f4f4f6
";

/// The same buttons in the dark theme, the icons light like the texts.
const DARK: &str = r"
             464    4 - #27272a
             592    4 - #27272a
             288   36 - #3a3a40
             300   36 - #e6e6e8
             312   36 - #3c3c42
             340   36 - #909094
              48   68 - #0a6cff
             152   72 - #0a6cff
              84   84 - #0a6cff
             104   88 - #0d6eff
             112   88 - #67a4ff
             300   88 - #444447
             316   88 - #515155
              16   92 - #0a6cff
              56  100 - #0a6cff
             484  100 - #27272a
             344  132 - #eeeef0
             204  136 - #f2f2f4
             216  136 - #89898b
             224  136 - #27272a
             240  136 - #78787b
             272  136 - #88888b
             288  136 - #b1b1b3
             300  136 - #27272a
             308  136 - #f2f2f4
             320  136 - #656568
             332  136 - #b1b1b3
             364  136 - #747477
             384  136 - #f2f2f4
             392  136 - #b1b1b3
             320  140 - #656568
             556  164 - #27272a
";
