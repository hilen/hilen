use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, LIGHT_GRAY, NumberView, Setup, Style, ViewData, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

const STYLE: Style = Style::new(|view| {
    view.apply_if::<NumberView>(|mut num| {
        num.set_labels("+", "–")
            .set_text_color(LIGHT_GRAY)
            .set_text_size(80)
            .set_gradient(BLUE, (0, 150, 150))
            .set_corner_radius(20);
    });
});

#[view]
struct NumberViewDesign {
    #[init]
    view: NumberView,
}

impl Setup for NumberViewDesign {
    fn setup(self: Weak<Self>) {
        self.view.place().tl(200).size(100, 200);
    }
}

impl ViewTest for NumberViewDesign {
    fn before_start() {
        from_main(|| {
            STYLE.apply_globally::<NumberView>();
        });
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // The style reset must run even when the check fails, or the
        // next application of this style panics as a duplicate.
        let check = check_colors(
            r"
               4    4 - #597c95
             592    4 - #597c95
             224  200 - #0000e7
             284  204 - #0003e5
             256  220 - #000fdf
             200  224 - #0012dd
             296  236 - #001bd8
             236  248 - #e7e7e7
             200  256 - #002ad0
             252  256 - #e7e7e7
             272  256 - #002ad0
             296  272 - #0036ca
             252  276 - #0039c8
             208  288 - #0042c3
             240  300 - #e7e7e7
             272  300 - #e7e7e7
             300  300 - #597c95
             592  300 - #597c95
             200  320 - #005ab6
             228  324 - #005db5
             256  324 - #005db5
             288  332 - #0063b1
             232  348 - #e7e7e7
             256  352 - #0072a9
             200  364 - #007ba4
             296  368 - #007ea3
             264  380 - #00879e
             208  392 - #009099
             236  396 - #009397
             288  396 - #009397
               4  592 - #597c95
             592  592 - #597c95
        ",
        );

        from_main(|| {
            STYLE.reset_global::<NumberView>();
        });

        check
    }
}
