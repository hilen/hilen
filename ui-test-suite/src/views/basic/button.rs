use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Button, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, state::increment_state, test_combinations},
};

#[view]
struct ButtonPress {
    #[init]
    button: Button,
}

impl Setup for ButtonPress {
    fn setup(self: Weak<Self>) {
        self.button.place().size(100, 50).t(25).l(50);
        self.button.set_text("Button text");

        self.button.on_tap(|| {
            increment_state();
        });
    }
}

impl ViewTest for ButtonPress {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        button_text_colors()?;
        tap_combinations()?;

        Ok(())
    }
}

fn button_text_colors() -> Result<()> {
    check_colors(
        r"
              52   28 - #ffffff
              92   28 - #ffffff
             128   28 - #ffffff
             108   44 - #000000
             128   44 - #000000
              56   48 - #000000
              68   48 - #ffffff
              76   48 - #bdbdbd
              88   48 - #ffffff
             100   48 - #bcbcbc
             144   48 - #ffffff
              76   52 - #bdbdbd
              92   52 - #ffffff
             100   52 - #bcbcbc
             128   52 - #000000
             132   52 - #ffffff
             140   52 - #010101
             144   52 - #010101
              56   56 - #000000
              64   56 - #020202
              84   56 - #010101
             100   56 - #bcbcbc
             112   56 - #000000
             144   56 - #ffffff
             144   60 - #010101
              52   72 - #ffffff
             128   72 - #ffffff
             592  136 - #597c95
              28  332 - #597c95
             300  384 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn tap_combinations() -> Result<()> {
    test_combinations([
        ("0 0 b", 0),
        ("0 0 e", 0),
        // Begin inside end outside
        ("100 50 b", 0),
        ("  0 50 e", 0),
        // Begin inside end outside
        ("100 50 b", 0),
        ("  0 50 e", 0),
        // Simple tap
        (
            r"
                100 50 b
                100 50 e
            ",
            1,
        ),
        // Simple tap
        (
            r"
                 90 50 b
                110 50 e
            ",
            1,
        ),
        // Outside then inside
        (
            r"
                  0 50 b
                110 50 e
            ",
            0,
        ),
        // Double release
        (
            r"
                 90 50 b
                110 50 e
                110 50 e
            ",
            1,
        ),
        (
            r"
                23.070313    49.19922     b
                85.86719     52.152344    e
                90.83594     12.671875    b
                89.625       49.941406    e
                184.75781    52.878906    b
                114.35547    48.38672     e
                101.80469    90.75391     b
                105.99219    49.027344    e
            ",
            0,
        ),
        (
            r"
                98.61328     48.339844    b
                0            0            m
                105.02344    50.539063    e

                0            0            m
                102.80469    49.39453     b
                0            0            m
                100.80078    47.55078     e

                0            0            m
                85.49219     50.351563    b
                0            0            m
                99.02734     49.777344    e
                ",
            3,
        ),
        (
            r"
                55.597656    32.632813    b
                55.660156    32.628906    e
                145.63281    33.753906    b
                145.33594    33.8125      e
                144.26172    73.14844     b
                144.19531    73.14844     e
                56.67578     72.02734     b
                56.632813    72.02734     e
                102.44531    50.621094    b
                102.37891    50.621094    e
                172.52344    49.304688    b
                171.8711     49.53125     e
                102.65234    92.15625     b
                102.19141    92.19141     e
                12.4140625   46.382813    b
                12.441406    46.382813    e
                102.51953    16.371094    b
                102.45703    16.199219    e
                ",
            5,
        ),
    ])
}
