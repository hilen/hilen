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
             412    4 - #597c95
             592    4 - #597c95
             204   40 - #007bff
              64   92 - #007bff
             148  100 - #007bff
             236  120 - #007bff
             320  136 - #2c7cca
             492  144 - #597c95
             124  180 - #2c7cca
             284  208 - #2c7cca
              92  216 - #2c7cca
             316  244 - #2c7cca
             184  252 - #007bff
              96  280 - #007bff
             128  288 - #2c7cca
             572  296 - #597c95
             280  316 - #2c7cca
              96  324 - #2c7cca
             176  356 - #007bff
             240  368 - #007bff
             436  428 - #597c95
              84  432 - #000000
             584  444 - #597c95
              36  448 - #000000
             132  456 - #000000
              12  496 - #000000
             148  504 - #000000
              28  544 - #000000
             124  548 - #000000
              76  568 - #000000
             340  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             412    4 - #597c95
             592    4 - #597c95
             204   40 - #007bff
              64   92 - #007bff
             148  100 - #007bff
             236  120 - #007bff
             320  136 - #2c7cca
             492  144 - #597c95
             124  180 - #2c7cca
             284  208 - #2c7cca
              92  216 - #2c7cca
             316  244 - #2c7cca
             184  252 - #007bff
              96  280 - #007bff
             128  288 - #2c7cca
             572  296 - #597c95
             280  316 - #2c7cca
              96  324 - #2c7cca
             176  356 - #007bff
             240  368 - #007bff
             436  428 - #597c95
              84  432 - #000000
             584  444 - #597c95
              36  448 - #000000
             132  456 - #000000
              12  496 - #000000
             148  504 - #000000
              28  544 - #000000
             124  548 - #000000
              76  568 - #000000
             340  592 - #597c95
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
             412    4 - #597c95
             592    4 - #597c95
             204   40 - #007bff
              64   92 - #007bff
             148  100 - #007bff
             236  120 - #007bff
             320  136 - #2c7cca
             492  144 - #597c95
             124  180 - #2c7cca
             284  208 - #2c7cca
              92  216 - #2c7cca
             316  244 - #2c7cca
             184  252 - #007bff
              96  280 - #007bff
             128  288 - #2c7cca
             572  296 - #597c95
             280  316 - #2c7cca
              96  324 - #2c7cca
             176  356 - #007bff
             240  368 - #007bff
             436  428 - #597c95
              84  432 - #0000e7
             584  444 - #597c95
              36  448 - #0000e7
             132  456 - #0000e7
              12  496 - #0000e7
             148  504 - #0000e7
              28  544 - #0000e7
             124  548 - #0000e7
              76  568 - #0000e7
             340  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
