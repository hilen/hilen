use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor::Top, BLUE, ImageView, Setup, Tinted, ViewData, ViewTest, ViewTouch, view},
    ui_test::helpers::check_colors,
};

#[view]
struct ImageViewSVG {
    #[init]
    bin:      ImageView,
    settings: ImageView,
}

impl Setup for ImageViewSVG {
    fn setup(self: Weak<Self>) {
        self.enable_touch();

        self.bin.place().tl(5).size(400, 400);
        self.bin.set_image("bin.svg");

        self.settings.place().same_x(self.bin).anchor(Top, self.bin, 20).size(150, 150);
        self.settings.set_image("settings.svg");
    }
}

impl ViewTest for ImageViewSVG {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_untinted_svg()?;
        check_tinted_settings(view)?;

        Ok(())
    }
}

fn check_untinted_svg() -> Result<()> {
    check_colors(
        r"
             592    4 - #597c95
             208   40 - #007bff
              72   88 - #375f81
             136   88 - #375f81
             276   88 - #375f81
             320  132 - #1e60a2
             188  172 - #225e9b
              92  212 - #2d5d8a
             188  228 - #225e9b
             316  232 - #1366bc
             316  236 - #1366bc
             316  240 - #1366bc
             316  244 - #1366bc
             224  272 - #007bff
             128  288 - #335e84
             592  308 - #597c95
              96  320 - #225e9b
             100  320 - #007bff
              96  324 - #225e9b
             312  324 - #007bff
             168  364 - #007bff
             248  368 - #007bff
             432  416 - #597c95
              96  448 - #000000
              28  456 - #000000
             148  496 - #000000
              32  524 - #000000
             132  544 - #000000
              88  564 - #000000
               4  592 - #597c95
             360  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             592    4 - #597c95
             208   40 - #007bff
              72   88 - #375f81
             136   88 - #375f81
             276   88 - #375f81
             320  132 - #1e60a2
             188  172 - #225e9b
              92  212 - #2d5d8a
             188  228 - #225e9b
             316  232 - #1366bc
             316  236 - #1366bc
             316  240 - #1366bc
             316  244 - #1366bc
             224  272 - #007bff
             128  288 - #335e84
             592  308 - #597c95
              96  320 - #225e9b
             100  320 - #007bff
              96  324 - #225e9b
             312  324 - #007bff
             168  364 - #007bff
             248  368 - #007bff
             432  416 - #597c95
              96  448 - #000000
              28  456 - #000000
             148  496 - #000000
              32  524 - #000000
             132  544 - #000000
              88  564 - #000000
               4  592 - #597c95
             360  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_tinted_settings(view: Weak<ImageViewSVG>) -> Result<()> {
    from_main(move || {
        view.settings.set_image(Tinted {
            tint: BLUE,
            name: "settings.svg".to_string(),
        });
    });

    check_colors(
        r"
             592    4 - #597c95
             208   40 - #007bff
              72   88 - #375f81
             136   88 - #375f81
             276   88 - #375f81
             320  132 - #1e60a2
             188  172 - #225e9b
              92  212 - #2d5d8a
             188  228 - #225e9b
             316  232 - #1366bc
             316  236 - #1366bc
             316  240 - #1366bc
             316  244 - #1366bc
             224  272 - #007bff
             128  288 - #335e84
             592  308 - #597c95
              96  320 - #225e9b
             100  320 - #007bff
              96  324 - #225e9b
             312  324 - #007bff
             168  364 - #007bff
             248  368 - #007bff
             432  416 - #597c95
              96  448 - #0000e7
              28  456 - #0000e7
             148  496 - #0000e7
              32  524 - #0000e7
             132  544 - #0000e7
              88  564 - #0000e7
               4  592 - #597c95
             360  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
