use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, GREEN, Setup, ViewData, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct NearLayout {
    #[init]
    base: Container,
    next: Container,
}

impl Setup for NearLayout {
    fn setup(self: Weak<Self>) {
        self.base.set_color(GREEN);
        self.next.set_color(BLUE);

        self.base.place().tl(20).size(50, 80);

        self.next.place().at_right(self.base, 20);
    }
}

impl ViewTest for NearLayout {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        check_at_right()?;
        check_below(view)?;
        check_at_right_with_width(view)?;
        check_below_with_width(view)?;
        check_below_with_height(view)?;

        Ok(())
    }
}

fn check_at_right() -> anyhow::Result<()> {
    check_colors(
        r"
             592    4 - #597c95
              24   24 - #00ff00
              52   24 - #00ff00
             104   24 - #0000e7
             136   24 - #0000e7
              68   32 - #00ff00
             116   44 - #0000e7
              40   48 - #00ff00
              64   48 - #00ff00
              92   48 - #0000e7
             136   52 - #0000e7
              52   60 - #00ff00
              24   68 - #00ff00
              68   72 - #00ff00
             100   72 - #0000e7
              44   76 - #00ff00
             124   76 - #0000e7
             368   76 - #597c95
              92   88 - #0000e7
              28   96 - #00ff00
              56   96 - #00ff00
             108   96 - #0000e7
             136   96 - #0000e7
             516  152 - #597c95
             300  300 - #597c95
             592  300 - #597c95
              56  348 - #597c95
             444  444 - #597c95
             152  496 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_below(view: Weak<NearLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.next.place().clear().below(view.base, 20);
    });

    check_colors(
        r"
             328    4 - #597c95
             592    4 - #597c95
              24   24 - #00ff00
              56   24 - #00ff00
              40   44 - #00ff00
              68   44 - #00ff00
              24   56 - #00ff00
              52   68 - #00ff00
              24   76 - #00ff00
              68   76 - #00ff00
              52   84 - #00ff00
              40   96 - #00ff00
              68   96 - #00ff00
              24  124 - #0000e7
              48  124 - #0000e7
              68  144 - #0000e7
             236  144 - #597c95
              24  148 - #0000e7
             460  152 - #597c95
              68  168 - #0000e7
              24  172 - #0000e7
              48  172 - #0000e7
              44  192 - #0000e7
              24  196 - #0000e7
              68  196 - #0000e7
             300  300 - #597c95
             592  300 - #597c95
             116  408 - #597c95
             444  444 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_at_right_with_width(view: Weak<NearLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.next.place().clear().at_right(view.base, 20).w(200);
    });

    check_colors(
        r"
             440    4 - #597c95
             592    4 - #597c95
              44   24 - #00ff00
              68   24 - #00ff00
              92   24 - #0000e7
             228   24 - #0000e7
              24   28 - #00ff00
             160   28 - #0000e7
             288   28 - #0000e7
              56   40 - #00ff00
              36   44 - #00ff00
             124   52 - #0000e7
             196   56 - #0000e7
             264   60 - #0000e7
              32   64 - #00ff00
              68   72 - #00ff00
              48   76 - #00ff00
             232   84 - #0000e7
             288   92 - #0000e7
              24   96 - #00ff00
              52   96 - #00ff00
             120   96 - #0000e7
             176   96 - #0000e7
             472  152 - #597c95
             300  300 - #597c95
             592  300 - #597c95
              56  348 - #597c95
             444  444 - #597c95
             152  496 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_below_with_width(view: Weak<NearLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.next.place().clear().below(view.base, 20).w(200);
    });

    check_colors(
        r"
             380    4 - #597c95
             592    4 - #597c95
              32   24 - #00ff00
              68   24 - #00ff00
              52   36 - #00ff00
              68   44 - #00ff00
              24   56 - #00ff00
              56   56 - #00ff00
              40   72 - #00ff00
              68   84 - #00ff00
              36   92 - #00ff00
              56   96 - #00ff00
              24  124 - #0000e7
             140  124 - #0000e7
             216  136 - #0000e7
              80  148 - #0000e7
             176  152 - #0000e7
             140  164 - #0000e7
             216  176 - #0000e7
             448  180 - #597c95
              80  188 - #0000e7
              40  196 - #0000e7
             116  196 - #0000e7
             184  196 - #0000e7
             300  300 - #597c95
             592  300 - #597c95
             112  408 - #597c95
             444  444 - #597c95
             152  556 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_below_with_height(view: Weak<NearLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.next.place().clear().below(view.base, 20).h(10);
    });

    check_colors(
        r"
             592    4 - #597c95
              24   24 - #00ff00
              68   24 - #00ff00
              48   28 - #00ff00
             332   40 - #597c95
              64   44 - #00ff00
              44   48 - #00ff00
              24   52 - #00ff00
              68   68 - #00ff00
              48   72 - #00ff00
              24   76 - #00ff00
              52   92 - #00ff00
              32   96 - #00ff00
              24  124 - #0000e7
              32  124 - #0000e7
              40  124 - #0000e7
              48  124 - #0000e7
              60  124 - #0000e7
              28  128 - #0000e7
              36  128 - #0000e7
              44  128 - #0000e7
              56  128 - #0000e7
              68  128 - #0000e7
             480  152 - #597c95
             300  300 - #597c95
             592  300 - #597c95
              72  364 - #597c95
             444  444 - #597c95
             152  512 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
