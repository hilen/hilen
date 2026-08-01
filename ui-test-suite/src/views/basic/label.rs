use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor, BLUE, LIGHTER_GRAY, Label, NumberView, Setup, TextAlignment, ViewData, ViewSubviews,
        ViewTest, WHITE, view,
    },
    ui_test::{helpers::check_colors, inject_touches},
};

#[view]
struct LabelSettings {
    #[init]
    label:          Label,
    text_size_view: NumberView,
}

impl Setup for LabelSettings {
    fn setup(self: Weak<Self>) {
        self.label.set_text("ßšėčыў").set_color(WHITE);
        self.label.place().size(280, 280).tl(80);

        self.text_size_view
            .place()
            .size(50, 50)
            .t(300)
            .anchor(Anchor::Right, self.label, 10);
        self.text_size_view.set_value(32.0).set_step(5.0);

        self.text_size_view.on_change(move |size| {
            self.label.set_text_size(size);
        });
    }
}

impl ViewTest for LabelSettings {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        initial_label_colors()?;
        stepper_changes_text_size()?;
        blue_text_color(view)?;
        left_right_aligned_labels(view)?;

        Ok(())
    }
}

fn initial_label_colors() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             120   84 - #ffffff
             356   84 - #ffffff
             240  100 - #ffffff
              84  176 - #ffffff
             356  200 - #ffffff
             176  212 - #ffffff
             192  216 - #ffffff
             228  216 - #ffffff
             232  220 - #ffffff
             176  224 - #ffffff
             192  224 - #ffffff
             244  224 - #ffffff
             592  260 - #597c95
              24  304 - #0096e6
              68  304 - #0096e6
              48  312 - #ffffff
              24  324 - #0096e6
              64  324 - #0096e6
              52  328 - #0096e6
              40  336 - #ffffff
              68  344 - #0096e6
              24  348 - #0096e6
              52  348 - #0096e6
             220  356 - #ffffff
             356  356 - #ffffff
             560  424 - #597c95
             424  524 - #597c95
               4  588 - #597c95
             256  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn stepper_changes_text_size() -> Result<()> {
    inject_touches(
        "
            39   305  b
            39   305  e
            41   301  b
            42   301  e
            42   301  b
            42   301  e
            42   301  b
            42   301  e
            42   301  b
            42   301  e
            42   301  b
            42   301  e
            42   301  b
            42   301  e
            42   301  b
            42   301  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            42   300  b
            42   300  e
            44   325  b
            44   325  e
            44   325  b
            44   325  e
            44   325  b
            44   325  e
            44   325  b
            44   325  e
            43   325  b
            43   325  e
            43   325  b
            43   325  e
            42   306  b
            43   308  e

        ",
    );

    check_colors(
        r"
             592    4 - #597c95
             120   84 - #ffffff
             240   84 - #ffffff
             356   84 - #ffffff
             348  184 - #000000
              96  188 - #000000
             196  188 - #000000
             236  200 - #010101
             148  208 - #ffffff
             196  208 - #ffffff
             116  212 - #000000
             184  220 - #010101
             328  220 - #010101
             280  224 - #ffffff
             152  232 - #ffffff
             208  232 - #000000
             104  240 - #000000
             240  240 - #000000
             324  256 - #010101
              24  304 - #0096e6
              68  304 - #0096e6
              48  312 - #ffffff
             592  316 - #597c95
              24  328 - #0096e6
              68  332 - #0096e6
              44  340 - #ffffff
              24  348 - #0096e6
             224  356 - #ffffff
             356  356 - #ffffff
               4  592 - #597c95
             260  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn blue_text_color(view: Weak<LabelSettings>) -> Result<()> {
    from_main(move || {
        view.label.set_text_color(BLUE);
    });

    check_colors(
        r"
             592    4 - #597c95
             120   84 - #ffffff
             252   84 - #ffffff
             356   84 - #ffffff
             232  184 - #0000e7
             348  184 - #0000e7
              96  188 - #0000e7
             196  188 - #0000e7
             308  204 - #0000e7
             148  208 - #ffffff
             116  212 - #0000e7
             184  220 - #0606e7
             280  224 - #ffffff
             344  224 - #0606e7
             152  232 - #ffffff
             196  232 - #ffffff
             104  240 - #0000e7
             236  240 - #0000e7
             324  256 - #0606e7
              24  304 - #0096e6
              68  304 - #0096e6
              48  312 - #ffffff
             592  316 - #597c95
              24  328 - #0096e6
              68  332 - #0096e6
              44  340 - #ffffff
              24  348 - #0096e6
             224  356 - #ffffff
             356  356 - #ffffff
               4  592 - #597c95
             260  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn left_right_aligned_labels(view: Weak<LabelSettings>) -> Result<()> {
    from_main(move || {
        view.label.set_text_size(28);
        view.add_view::<Label>()
            .set_text("Left Left")
            .set_alignment(TextAlignment::Left)
            .set_color(LIGHTER_GRAY)
            .place()
            .tl(60)
            .w(200)
            .h(60);
        view.add_view::<Label>()
            .set_text("Right")
            .set_alignment(TextAlignment::Right)
            .set_color(LIGHTER_GRAY)
            .place()
            .l(60)
            .w(200)
            .t(280)
            .h(60);
    });

    check_colors(
        r"
             592    4 - #597c95
              64   64 - #f3f3f3
             212   64 - #f3f3f3
             356   84 - #ffffff
             504  132 - #597c95
             276  136 - #ffffff
              84  168 - #ffffff
             180  212 - #ffffff
             196  216 - #ffffff
             232  220 - #ffffff
             356  228 - #ffffff
             592  260 - #597c95
             296  292 - #ffffff
             180  300 - #f3f3f3
              24  304 - #0096e6
              68  304 - #0096e6
              36  308 - #0096e6
             208  308 - #f3f3f3
              48  312 - #ffffff
             208  320 - #f3f3f3
              64  324 - #0096e6
              32  328 - #0096e6
              52  328 - #0096e6
              44  340 - #ffffff
              64  344 - #0096e6
              24  348 - #0096e6
             356  356 - #ffffff
             520  428 - #597c95
             168  476 - #597c95
              44  592 - #597c95
             292  592 - #597c95
             592  592 - #597c95
        ",
    )
}
