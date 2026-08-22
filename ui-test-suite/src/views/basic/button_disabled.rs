use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Button, DynamicColor, Setup, Theme, ThemeMode, UIColor, ViewData, ViewTest, view},
    ui_test::{
        check_colors, inject_touches,
        state::{get_state, increment_state},
    },
};

#[view]
struct ButtonDisabled {
    #[init]
    button: Button,
}

impl Setup for ButtonDisabled {
    fn setup(self: Weak<Self>) {
        self.button.place().size(200, 60).t(40).center_x();
        self.button.set_text("Add");
        self.button.set_color("#3b82f6");
        self.button.set_text_color("#ffffff");

        self.button.on_tap(|| {
            increment_state();
        });
    }
}

impl ViewTest for ButtonDisabled {
    fn canvas() -> (u32, u32) {
        (600, 200)
    }

    /// Pinned before the view is built, so the recorded colors do not
    /// depend on whatever theme the OS happens to be in. Headless has no
    /// window to read one from and lands on light, a real window follows
    /// the desktop, and the two disagreed.
    fn before_start() {
        from_main(|| {
            Theme::set_mode(ThemeMode::Light);
        });
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert!(view.button.is_enabled());

        enabled_colors()?;

        tap();
        assert_eq!(get_state::<u32>(), 1);

        from_main(move || {
            let mut button = view.button;
            button.set_enabled(false);
        });
        assert!(!view.button.is_enabled());

        disabled_colors()?;

        // The whole point: a disabled button swallows the tap.
        tap();
        assert_eq!(get_state::<u32>(), 1);

        from_main(move || {
            let mut button = view.button;
            button.set_enabled(true);
        });
        assert!(view.button.is_enabled());

        // Re-enabling has to put the original colors back, not leave the
        // disabled gray behind.
        enabled_colors()?;

        tap();
        assert_eq!(get_state::<u32>(), 2);

        // A theme pair has to survive the disable and enable trip. Storing
        // the resolved color instead of the pair would leave the button
        // stuck on whatever the theme was when it got disabled.
        from_main(move || {
            let mut button = view.button;
            button.set_color(UIColor::Dynamic(DynamicColor::new(
                "#3b82f6".into(),
                "#1d4ed8".into(),
            )));
            button.set_enabled(false);
            button.set_enabled(true);
            Theme::set_mode(ThemeMode::Dark);
            assert_eq!(Theme::current(), Theme::Dark);
        });

        wait_for_next_frame();
        dark_after_round_trip()?;

        from_main(|| {
            Theme::set_mode(ThemeMode::System);
        });

        Ok(())
    }
}

/// Taps the button's upper right, well inside it but away from every
/// recorded probe. This test has to alternate taps and color checks,
/// unlike `ButtonPress` which does all its checks first, and human mode
/// draws a marker over each injected touch. A tap on the button centre
/// put four stacked markers on top of the probe at 300 72.
fn tap() {
    inject_touches(
        r"
        350  50   b
        350  50   e
    ",
    );
}

fn enabled_colors() -> Result<()> {
    check_colors(ENABLED)
}

fn disabled_colors() -> Result<()> {
    check_colors(DISABLED)
}

fn dark_after_round_trip() -> Result<()> {
    check_colors(DARK_AFTER_ROUND_TRIP)
}

/// The colors the app set, a blue fill under white text.
const ENABLED: &str = r"
              16    4 - #597c95
             584    4 - #597c95
             244   40 - #3b82f6
             396   40 - #3b82f6
             200   44 - #3b82f6
             308   60 - #609af8
             324   60 - #ffffff
             284   64 - #ffffff
             308   64 - #609af8
             324   64 - #ffffff
             300   68 - #3b82f6
             308   68 - #609af8
             324   68 - #ffffff
             276   72 - #ffffff
             280   72 - #9abefa
             300   72 - #3b82f6
             304   72 - #4186f6
             308   72 - #609af8
             316   72 - #3b82f6
             324   72 - #ffffff
             288   76 - #ffffff
             296   76 - #fcfdff
             300   76 - #3b82f6
             308   76 - #609af8
             320   76 - #3b82f6
             324   76 - #ffffff
             300   80 - #ffffff
             224   96 - #3b82f6
             380   96 - #3b82f6
               4  192 - #597c95
             136  192 - #597c95
             592  192 - #597c95
        ";

/// The default disabled pair, gray fill under dimmer gray text.
const DISABLED: &str = r"
              16    4 - #597c95
             584    4 - #597c95
             244   40 - #d6d6d8
             396   40 - #d6d6d8
             200   44 - #d6d6d8
             324   60 - #8e8e93
             364   60 - #d6d6d8
             284   64 - #8e8e93
             300   64 - #8e8e93
             324   64 - #8e8e93
             296   68 - #8f8f94
             300   68 - #d6d6d8
             324   68 - #8e8e93
             276   72 - #8e8e93
             280   72 - #b3b3b7
             300   72 - #d6d6d8
             304   72 - #d4d4d6
             316   72 - #d6d6d8
             320   72 - #d6d6d8
             324   72 - #8e8e93
             288   76 - #8e8e93
             296   76 - #8f8f94
             300   76 - #d6d6d8
             320   76 - #d6d6d8
             324   76 - #8e8e93
             300   80 - #8e8e93
             224   96 - #d6d6d8
             376   96 - #d6d6d8
               4  192 - #597c95
             136  192 - #597c95
             464  192 - #597c95
             592  192 - #597c95
        ";

/// The dark half of the pair, `#1d4ed8`. Capturing the resolved color
/// instead of the pair would leave the button on the light `#3b82f6`
/// here, which is the regression this check exists for.
const DARK_AFTER_ROUND_TRIP: &str = r"
              16    4 - #597c95
             584    4 - #597c95
             244   40 - #1d4ed8
             396   40 - #1d4ed8
             200   44 - #1d4ed8
             308   60 - #4870df
             324   60 - #ffffff
             284   64 - #ffffff
             308   64 - #4870df
             324   64 - #ffffff
             300   68 - #1d4ed8
             308   68 - #4870df
             324   68 - #ffffff
             276   72 - #ffffff
             280   72 - #8aa3eb
             300   72 - #1d4ed8
             304   72 - #2454d9
             308   72 - #4870df
             316   72 - #1d4ed8
             324   72 - #ffffff
             288   76 - #ffffff
             296   76 - #fcfcfe
             300   76 - #1d4ed8
             308   76 - #4870df
             320   76 - #1d4ed8
             324   76 - #ffffff
             300   80 - #ffffff
             224   96 - #1d4ed8
             380   96 - #1d4ed8
               4  192 - #597c95
             136  192 - #597c95
             592  192 - #597c95
        ";
