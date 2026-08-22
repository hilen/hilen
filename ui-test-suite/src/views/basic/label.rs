use anyhow::Result;
use hilen::{
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
             124   80 - #ffffff
             356   80 - #ffffff
             168  212 - #010101
             168  216 - #010101
             196  216 - #010101
             256  216 - #606060
             168  220 - #000000
             220  220 - #000000
             240  220 - #000000
             256  220 - #606060
             168  224 - #000000
             212  224 - #ffffff
             228  224 - #ffffff
             256  224 - #606060
             260  224 - #ffffff
             168  228 - #010101
             180  228 - #010101
             256  228 - #606060
             592  260 - #597c95
              20  304 - #0096e6
              68  304 - #0096e6
              44  308 - #ffffff
              64  324 - #0096e6
              36  332 - #ffffff
              48  340 - #ffffff
              68  348 - #0096e6
             220  356 - #ffffff
             356  356 - #ffffff
             272  592 - #597c95
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
             280   80 - #ffffff
              80   84 - #ffffff
             180   88 - #ffffff
              96  188 - #000000
             160  188 - #000000
             228  188 - #010101
             332  188 - #2c2c2c
             196  192 - #010101
             268  204 - #b4b4b4
             308  204 - #b4b4b4
             328  204 - #b4b4b4
             356  204 - #b4b4b4
             108  220 - #010101
             188  228 - #8d8d8d
             200  228 - #8d8d8d
             208  228 - #8d8d8d
             152  236 - #ffffff
             280  236 - #ffffff
              88  244 - #010101
             236  244 - #000000
             324  260 - #8d8d8d
             328  260 - #010101
              64  300 - #0096e6
              20  304 - #0096e6
              44  312 - #ffffff
             592  324 - #597c95
              68  328 - #0096e6
              44  340 - #ffffff
             228  356 - #ffffff
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
             280   80 - #ffffff
              80   84 - #ffffff
             180   88 - #ffffff
              96  188 - #0000e7
             160  188 - #0000e7
             228  188 - #0101e7
             332  188 - #2c2ceb
             196  192 - #0101e7
             268  204 - #b4b4f8
             308  204 - #b4b4f8
             328  204 - #b4b4f8
             356  204 - #b4b4f8
             108  220 - #0101e7
             188  228 - #8d8df4
             200  228 - #8d8df4
             208  228 - #8d8df4
             152  236 - #ffffff
             280  236 - #ffffff
              88  244 - #0101e7
             236  244 - #0000e7
             324  260 - #8d8df4
             328  260 - #0101e7
              64  300 - #0096e6
              20  304 - #0096e6
              44  312 - #ffffff
             592  324 - #597c95
              68  328 - #0096e6
              44  340 - #ffffff
             228  356 - #ffffff
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
              80   80 - #000000
             144   80 - #000000
             356   80 - #ffffff
             116   88 - #454545
             188   88 - #484848
             116   92 - #454545
             180   92 - #858585
              84  100 - #0f0f0f
             152  100 - #0f0f0f
             176  216 - #5e5ef0
             236  216 - #0000e7
             176  220 - #5e5ef0
             176  224 - #5e5ef0
             256  224 - #ffffff
             180  228 - #0101e7
             212  228 - #0101e7
             236  228 - #0101e7
             592  280 - #597c95
             176  304 - #dddddd
              44  308 - #ffffff
             192  308 - #919191
             236  308 - #bbbbbb
             212  312 - #000000
             236  312 - #bbbbbb
             176  316 - #dddddd
             192  316 - #919191
             228  316 - #7a7a7a
              44  336 - #ffffff
              20  344 - #0096e6
              68  348 - #0096e6
             356  356 - #ffffff
             592  592 - #597c95
        ",
    )
}
