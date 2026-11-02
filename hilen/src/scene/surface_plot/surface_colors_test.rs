use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::Color,
    scene::SurfacePlotView,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// A roof `z = 4 - |x|` that takes its colors from the function, not from
/// its height: green where x is below 0 and magenta above, so both sides
/// of the ridge have the same heights and still differ.
#[view]
struct SurfaceColors {
    #[init]
    status: Label,
    plot:   SurfacePlotView,
}

impl Setup for SurfaceColors {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status
            .set_text("a roof, green left of the ridge, magenta right")
            .set_text_size(20)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.plot.place().t(52).lr(12).b(12);
        self.plot.set_colored_function(|x, _| {
            let color = if x < 0.0 {
                Color::hex("#2D8A3E")
            } else {
                Color::hex("#B5179E")
            };
            (4.0 - x.abs(), color)
        });
    }
}

impl ViewTest for SurfaceColors {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();
        wait_for_next_frame();
        assert!(from_main(move || view.plot.has_surface()));
        check_colors(CHECK_1)?;
        Ok(())
    }
}

const CHECK_1: &str = r"
     156   16 - #1b1f24
     284   20 - #1b1f24
     324   20 - #9b9d9f
     432   20 - #1b1f24
     460   20 - #202429
     488   20 - #1b1f24
     136   24 - #ffffff
     188   24 - #1b1f24
     216   24 - #1b1f24
     248   24 - #7c7e81
     324   24 - #9b9d9f
     384   24 - #d6d7d7
     484   24 - #bebfc0
     300  148 - #46a758
     152  168 - #f4f5f7
     300  180 - #46a758
     592  192 - #ffffff
       4  204 - #ffffff
     300  212 - #46a758
     300  228 - #46a758
     368  240 - #ac2d97
     300  244 - #46a758
     300  260 - #46a758
     288  272 - #f4f5f7
     236  288 - #f4f5f7
     188  296 - #f4f5f7
     412  296 - #ac2c97
     392  308 - #ac2c97
     468  312 - #f4f5f7
     148  316 - #f4f5f7
     416  316 - #ac2c97
     436  320 - #c7ccd4
     188  328 - #234d29
     380  328 - #ac2c97
     436  332 - #ac2c97
     348  336 - #e5484d
     408  336 - #ac2c97
     268  340 - #ac2d97
     172  344 - #234d29
     368  344 - #e5484d
     392  352 - #e5484d
     188  356 - #234d29
     160  360 - #234d29
     412  360 - #e5484d
     436  368 - #e5484d
     216  372 - #3e8ed0
     148  380 - #234d29
     204  380 - #3e8ed0
     472  380 - #e5484d
     196  384 - #3e8ed0
     368  384 - #b8bec8
     128  392 - #234d29
     184  392 - #3e8ed0
     172  400 - #3e8ed0
     164  404 - #3e8ed0
     240  404 - #f4f5f7
     344  436 - #ac2c97
     288  448 - #ac2d97
     392  468 - #ac2c97
     336  540 - #ac2d97
     172  580 - #f4f5f7
       4  592 - #ffffff
     592  592 - #ffffff
";
