use std::env::temp_dir;

use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{Container, GREEN, Setup, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, recording_colors},
};

#[view]
struct TestColorChecker {
    #[init]
    view: Container,
}

impl Setup for TestColorChecker {
    fn setup(self: Weak<Self>) {
        self.view.set_frame((80, 200, 20, 20)).set_color(GREEN);
    }
}

impl ViewTest for TestColorChecker {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
            4    4 -  89 124 149
            304    4 -  89 124 149
            592    4 -  89 124 149
            156   56 -  89 124 149
            444  108 -  89 124 149
            280  188 -  89 124 149
            592  200 -  89 124 149
            84  204 -   0 255   0
            88  204 -   0 255   0
            92  204 -   0 255   0
            96  204 -   0 255   0
            84  208 -   0 255   0
            88  208 -   0 255   0
            92  208 -   0 255   0
            96  208 -   0 255   0
            84  212 -   0 255   0
            88  212 -   0 255   0
            92  212 -   0 255   0
            96  212 -   0 255   0
            84  216 -   0 255   0
            88  216 -   0 255   0
            92  216 -   0 255   0
            96  216 -   0 255   0
            424  300 -  89 124 149
            96  352 -  89 124 149
            228  380 -  89 124 149
            592  396 -  89 124 149
            4  448 -  89 124 149
            396  512 -  89 124 149
            48  592 -  89 124 149
            200  592 -  89 124 149
            592  592 -  89 124 149
        ",
        )?;

        // These assertions inspect the error text of a deliberately failing
        // check. Record mode never fails checks, so they must not run there
        // or the whole record pass dies on this test.
        if !recording_colors() {
            let error = check_colors(
                r"
                  76  215 -  89 124 149
                  90  214 -   0   0 255
                 112  213 -  89 124 149
            ",
            )
            .err()
            .unwrap()
            .to_string();

            assert!(error.starts_with(
                r"
        Test: Test color checker has failed.
        Color diff is too big: 510. Max: 45. Position: Point { x: 90.0, y: 214.0 }.
        Expected: r: 0, g: 0, b: 255, a: 255, got: r: 0, g: 255, b: 0, a: 255.
          90  214 -   0   0 255 ->   0 255   0"
            ));

            if cfg!(target_arch = "wasm32") {
                // The browser report skips the file, see report.rs.
                assert!(error.contains("no filesystem"));
            } else {
                let screenshot_path = temp_dir().join("ui_test_Test_color_checker.png");

                assert!(error.contains(&format!("Failure screenshot: {}", screenshot_path.display())));
                assert!(screenshot_path.exists());
            }
            assert!(error.contains("View tree"));
        }

        check_colors(
            r"
            4    4 -  89 124 149
            304    4 -  89 124 149
            592    4 -  89 124 149
            156   56 -  89 124 149
            444  108 -  89 124 149
            280  188 -  89 124 149
            592  200 -  89 124 149
            84  204 -   0 255   0
            88  204 -   0 255   0
            92  204 -   0 255   0
            96  204 -   0 255   0
            84  208 -   0 255   0
            88  208 -   0 255   0
            92  208 -   0 255   0
            96  208 -   0 255   0
            84  212 -   0 255   0
            88  212 -   0 255   0
            92  212 -   0 255   0
            96  212 -   0 255   0
            84  216 -   0 255   0
            88  216 -   0 255   0
            92  216 -   0 255   0
            96  216 -   0 255   0
            424  300 -  89 124 149
            96  352 -  89 124 149
            228  380 -  89 124 149
            592  396 -  89 124 149
            4  448 -  89 124 149
            396  512 -  89 124 149
            48  592 -  89 124 149
            200  592 -  89 124 149
            592  592 -  89 124 149
        ",
        )?;

        Ok(())
    }
}
