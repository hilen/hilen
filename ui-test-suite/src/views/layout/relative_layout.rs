use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, GREEN, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct RelativeLayout {
    view: Weak<Container>,

    #[init]
    parent: Container,
}

impl Setup for RelativeLayout {
    fn setup(mut self: Weak<Self>) {
        self.parent.set_color(BLUE);
        self.parent.set_frame((50, 50, 200, 200));

        self.view = self.parent.add_view();

        self.view.set_color(GREEN);
        self.view
            .place()
            .relative_size(self.parent, 0.4)
            .relative_x(0.2)
            .relative_y(0.5);
    }
}

impl ViewTest for RelativeLayout {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             392    4 - #597c95
             592    4 - #597c95
              52   52 - #0000e7
             152   52 - #0000e7
             248   72 - #0000e7
             192   96 - #0000e7
             104  100 - #0000e7
              52  104 - #0000e7
             236  140 - #0000e7
              92  152 - #00ff00
             168  152 - #00ff00
             448  152 - #597c95
              52  156 - #0000e7
             140  160 - #00ff00
             112  168 - #00ff00
             160  180 - #00ff00
             136  192 - #00ff00
              92  204 - #00ff00
             168  204 - #00ff00
             248  212 - #0000e7
             124  228 - #00ff00
             152  228 - #00ff00
              52  248 - #0000e7
             204  248 - #0000e7
             300  300 - #597c95
             592  300 - #597c95
             144  436 - #597c95
             444  444 - #597c95
             152  584 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        from_main(move || {
            view.parent.set_size(280, 400);
        });

        check_colors(
            r"
             432    4 - #597c95
             592    4 - #597c95
              52   52 - #0000e7
             132   52 - #0000e7
             228   52 - #0000e7
             304   52 - #0000e7
             180  112 - #0000e7
              96  120 - #0000e7
             468  136 - #597c95
             288  180 - #0000e7
             208  184 - #0000e7
              52  188 - #0000e7
             168  252 - #00ff00
             108  260 - #00ff00
             592  268 - #597c95
             216  292 - #00ff00
             332  300 - #597c95
             112  320 - #00ff00
             168  320 - #00ff00
             208  356 - #00ff00
             140  364 - #00ff00
             328  388 - #0000e7
             216  400 - #00ff00
             592  404 - #597c95
             108  408 - #00ff00
             172  408 - #00ff00
             464  444 - #597c95
             268  448 - #0000e7
               4  592 - #597c95
             176  592 - #597c95
             368  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
