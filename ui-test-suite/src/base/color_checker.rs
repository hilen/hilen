use std::env::temp_dir;

use anyhow::Result;
use hilen::{
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
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
             156   56 - #597c95
             444  108 - #597c95
             280  188 - #597c95
             592  200 - #597c95
              84  204 - #00ff00
              88  204 - #00ff00
              92  204 - #00ff00
              96  204 - #00ff00
              84  208 - #00ff00
              88  208 - #00ff00
              92  208 - #00ff00
              96  208 - #00ff00
              84  212 - #00ff00
              88  212 - #00ff00
              92  212 - #00ff00
              96  212 - #00ff00
              84  216 - #00ff00
              88  216 - #00ff00
              92  216 - #00ff00
              96  216 - #00ff00
             424  300 - #597c95
              96  352 - #597c95
             228  380 - #597c95
             592  396 - #597c95
               4  448 - #597c95
             396  512 - #597c95
              48  592 - #597c95
             200  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        // These assertions inspect the error text of a deliberately failing
        // check. Record mode never fails checks, so they must not run there
        // or the whole record pass dies on this test.
        if !recording_colors() {
            let error = check_colors(
                r"
              76  215 - #597c95
              90  214 - #0000ff
             112  213 - #597c95
            ",
            )
            .err()
            .unwrap()
            .to_string();

            assert!(error.starts_with(
                r"
        Test: Test color checker has failed.
        Color diff is too big: 510. Max: 45. Position: Point { x: 90.0, y: 214.0 }.
        Expected: #0000ff, got: #00ff00.
          90  214 - #0000ff -> #00ff00"
            ));

            if cfg!(target_arch = "wasm32") {
                // The browser report pushes the frame to the driver, see report.rs.
                assert!(error.contains("sent to the driver"));
            } else {
                let screenshot_path = temp_dir().join("ui_test_Test_color_checker.png");

                assert!(error.contains(&format!("Failure screenshot: {}", screenshot_path.display())));
                assert!(screenshot_path.exists());
            }
            assert!(error.contains("View tree"));
        }

        check_colors(
            r"
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
             156   56 - #597c95
             444  108 - #597c95
             280  188 - #597c95
             592  200 - #597c95
              84  204 - #00ff00
              88  204 - #00ff00
              92  204 - #00ff00
              96  204 - #00ff00
              84  208 - #00ff00
              88  208 - #00ff00
              92  208 - #00ff00
              96  208 - #00ff00
              84  212 - #00ff00
              88  212 - #00ff00
              92  212 - #00ff00
              96  212 - #00ff00
              84  216 - #00ff00
              88  216 - #00ff00
              92  216 - #00ff00
              96  216 - #00ff00
             424  300 - #597c95
              96  352 - #597c95
             228  380 - #597c95
             592  396 - #597c95
               4  448 - #597c95
             396  512 - #597c95
              48  592 - #597c95
             200  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
