use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    ui::{Label, PlotView, Setup, ViewData, ViewTest, view},
};

/// Functions whose values are far past what an f32 holds, like `x^100`
/// away from 0, used to panic in the convert to a screen point. They
/// draw the part that fits and leave the rest out.
#[view]
struct PlotHugeValues {
    #[init]
    status: Label,
    plot:   PlotView,
}

impl Setup for PlotHugeValues {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status
            .set_text("x^100, 1e300 * x and 1/x draw with no panic")
            .set_text_size(20)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.plot.place().t(52).lr(12).b(12);
        self.plot
            .set_x_range(-10.0, 10.0, 0.0)
            .add_function(|x| x.powi(100), "#1F5FBF")
            .add_function(|x| 1e300 * x, "#D7263D")
            .add_function(|x| 1.0 / x, "#2D8A3E")
            .add_points([(-5.0, -1e200), (0.0, 0.0), (5.0, 1e200)], "#7B2CBF");
    }
}

impl ViewTest for PlotHugeValues {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        from_main(move || {
            view.plot.zoom_at((300, 300), 0.001);
        });
        wait_for_next_frame();
        let range = from_main(move || view.plot.x_range());
        assert!(range.1 - range.0 > 10_000.0, "{range:?}");
        Ok(())
    }
}
