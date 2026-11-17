use anyhow::{Result, ensure};
#[cfg(any(desktop, wasm))]
use hilen::ui::ViewTouch;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    text::Font,
    ui::{
        Anchor::Left, Container, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ToolButton,
        ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#F4F5F7"), Color::hex("#1B1D21"));
const INK: DynamicColor = DynamicColor::new(Color::hex("#20232A"), Color::hex("#E8EAEE"));
const MARK: DynamicColor = DynamicColor::new(Color::hex("#C2410C"), Color::hex("#FB923C"));
const ON: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#2A2D33"));
const HOVER: DynamicColor = DynamicColor::new(Color::hex("#E6E8EC"), Color::hex("#30333A"));
const PRESSED: DynamicColor = DynamicColor::new(Color::hex("#D9DCE2"), Color::hex("#3A3E46"));

/// An icon the app built in, drawn in black.
const ICON: &[u8] = include_bytes!("tool_button_bytes.svg");

const IDLE: &str = r"
             380    4 - #f4f5f7
              60   24 - #20232a
             196   24 - #393c42
             204   24 - #20232a
             208   24 - #606268
              60   28 - #20232a
              64   28 - #f4f5f7
             100   28 - #f4f5f7
             128   28 - #f4f5f7
             136   28 - #f4f5f7
             148   28 - #f4f5f7
             192   28 - #f4f5f7
             196   28 - #20232a
             204   28 - #20232a
             220   28 - #8a8c91
             264   28 - #f4f5f7
             272   28 - #4e5056
             324   28 - #f4f5f7
              36   32 - #63656a
             372   60 - #f4f5f7
             136   72 - #20232a
              60   76 - #c75121
              68   76 - #c2410c
             136   76 - #20232a
             140   76 - #20232a
             152   76 - #87898e
              52   80 - #c2410c
             316  100 - #f4f5f7
               4  112 - #f4f5f7
             192  112 - #f4f5f7
             244  112 - #f4f5f7
             392  112 - #f4f5f7
";
#[cfg(any(desktop, wasm))]
const SECOND_HOVERED: &str = r"
             380    4 - #f4f5f7
              60   24 - #20232a
             196   24 - #393c42
             204   24 - #20232a
             208   24 - #606268
              60   28 - #20232a
              64   28 - #f4f5f7
             100   28 - #f4f5f7
             128   28 - #f4f5f7
             136   28 - #f4f5f7
             148   28 - #f4f5f7
             192   28 - #f4f5f7
             196   28 - #20232a
             204   28 - #20232a
             220   28 - #8a8c91
             264   28 - #f4f5f7
             272   28 - #4e5056
             324   28 - #f4f5f7
              36   32 - #63656a
             372   60 - #f4f5f7
             136   72 - #20232a
              60   76 - #c75121
              68   76 - #c2410c
             136   76 - #20232a
             140   76 - #20232a
             152   76 - #808389
              52   80 - #c2410c
             316  100 - #f4f5f7
               4  112 - #f4f5f7
             192  112 - #f4f5f7
             244  112 - #f4f5f7
             392  112 - #f4f5f7
";
const SECOND_ON: &str = r"
             380    4 - #f4f5f7
              60   24 - #20232a
             196   24 - #393c42
             204   24 - #20232a
             208   24 - #606268
              60   28 - #20232a
              64   28 - #f4f5f7
             100   28 - #f4f5f7
             128   28 - #f4f5f7
             136   28 - #f4f5f7
             148   28 - #f4f5f7
             192   28 - #f4f5f7
             196   28 - #20232a
             204   28 - #20232a
             220   28 - #8a8c91
             264   28 - #f4f5f7
             272   28 - #4e5056
             324   28 - #f4f5f7
              36   32 - #63656a
             372   60 - #f4f5f7
             136   72 - #c2410c
              60   76 - #32353b
              68   76 - #20232a
             136   76 - #c2410c
             140   76 - #c2410c
             152   76 - #e09d82
              52   80 - #20232a
             316  100 - #f4f5f7
               4  112 - #f4f5f7
             192  112 - #f4f5f7
             244  112 - #f4f5f7
             392  112 - #f4f5f7
";

/// 2 tabs of `ToolButton` in the colors and the font of an app, with an icon
/// from bytes. The picked tab has the app's own fill and mark, a hover and a
/// press the app's own fills.
#[view]
struct ToolButtonLook {
    #[init]
    page:   Container,
    title:  Label,
    first:  ToolButton,
    second: ToolButton,
}

impl ToolButtonLook {
    fn pick(self: Weak<Self>, second: bool) {
        self.first.set_on(!second);
        self.second.set_on(second);
    }
}

impl Setup for ToolButtonLook {
    fn setup(self: Weak<Self>) {
        self.page.set_color(PAGE);
        self.page.place().back();

        self.title
            .set_text_size(16)
            .set_alignment(TextAlignment::Left)
            .set_text_color(INK);
        self.title.set_text("2 tabs in the colors and the font of an app");
        self.title.place().t(16).lr(16).h(24);

        for (tab, text) in [(self.first, "Bugs"), (self.second, "Ideas")] {
            tab.set_icon_data(ICON, "tool_look_tab.svg").set_text(text);
            tab.set_font(Font::medium()).set_text_size(16);
            tab.set_tint(INK).set_on_tint(MARK);
            tab.set_on_color(ON).set_hover_color(HOVER).set_pressed_color(PRESSED);
        }
        self.first.place().t(56).l(16).h(40);
        self.second.place().t(56).anchor(Left, self.first, 8).h(40);

        self.first.on_tap(move || self.pick(false));
        self.second.on_tap(move || self.pick(true));
        self.pick(false);
    }
}

impl ViewTest for ToolButtonLook {
    fn canvas() -> (u32, u32) {
        (400, 120)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("390 110 m");
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                view.first.is_on() && !view.second.is_on(),
                "the first tab is not the picked one"
            );
            ensure!(
                *view.first.color() == ON.resolve(),
                "the picked tab has no fill of the app"
            );
            ensure!(
                (view.first.width() - view.first.fit_width()).abs() < 0.01,
                "the width {} is not the fit width {}",
                view.first.width(),
                view.first.fit_width()
            );
            Ok(())
        })?;
        check_colors(IDLE)?;

        let (x, y) = from_main(move || {
            let frame = view.second.absolute_frame();
            (frame.center().x, frame.center().y)
        });
        // Hover needs a pointer, a phone has none, the gate of `hover` in the
        // module list. The press and the pick below are on every platform.
        #[cfg(any(desktop, wasm))]
        {
            inject_touches(format!("{x} {y} m"));
            wait_for_next_frame();
            from_main(move || {
                ensure!(view.second.is_hovered(), "the second tab took no hover");
                ensure!(
                    *view.second.color() == HOVER.resolve(),
                    "the hover is not the fill of the app"
                );
                Ok(())
            })?;
            check_colors(SECOND_HOVERED)?;
        }

        inject_touches(format!("{x} {y} b"));
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                *view.second.color() == PRESSED.resolve(),
                "the press is not the fill of the app"
            );
            Ok(())
        })?;
        inject_touches(format!("{x} {y} e\n390 110 m"));
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                view.second.is_on() && !view.first.is_on(),
                "the tap did not pick the second tab"
            );
            Ok(())
        })?;
        check_colors(SECOND_ON)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}
