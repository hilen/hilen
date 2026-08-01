use anyhow::Result;
use test_engine::{
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
             272  200 - #0000e7
             208  208 - #0006e4
             240  216 - #000ce0
             296  224 - #0012dd
             268  248 - #e7e7e7
             240  252 - #e7e7e7
             296  256 - #002ad0
             200  268 - #0033cb
             256  276 - #0039c8
             228  280 - #003cc6
             220  300 - #e7e7e7
             272  300 - #e7e7e7
             588  300 - #597c95
             248  304 - #004ebd
             200  312 - #0054b9
             296  312 - #0054b9
             232  324 - #005db5
             272  348 - #539ac1
             232  352 - #4f9abe
             244  352 - #4f9abe
             252  352 - #4f9abe
             264  352 - #4f9abe
             200  360 - #0078a6
             296  364 - #007ba4
             272  376 - #0084a0
             208  392 - #009099
             248  392 - #009099
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
