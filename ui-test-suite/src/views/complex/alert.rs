use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{Alert, RED, TextAlignment, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct AlertTestView {}

fn check_alert_shown() -> Result<()> {
    from_main(|| {
        Alert::show("Forogorn\nSopokok\nFergel");
    });

    check_colors(
        r"
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
             156   48 - #597c95
             164  204 - #ffffff
             368  204 - #ffffff
             436  208 - #ffffff
             256  244 - #ffffff
             292  248 - #ffffff
             324  252 - #ffffff
             252  272 - #ffffff
               4  280 - #597c95
             188  280 - #ffffff
             272  280 - #ffffff
             300  280 - #ffffff
             332  280 - #ffffff
             400  288 - #ffffff
             592  288 - #597c95
             272  300 - #ffffff
             308  308 - #ffffff
             240  348 - #ffffff
             164  352 - #ffffff
             364  356 - #ffffff
             436  364 - #ffffff
             284  380 - #ffffff
             300  392 - #ffffff
             204  396 - #ffffff
              28  436 - #597c95
             404  552 - #597c95
               4  592 - #597c95
             220  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_alert_dismissed() -> Result<()> {
    inject_touches(
        "
            338  373  b
            338  373  e
        ",
    );

    check_colors(
        r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_styled_alert() -> Result<()> {
    from_main(|| {
        Alert::with_label(|l| {
            l.set_text_color(RED).set_text_size(50).set_alignment(TextAlignment::Left);
        })
        .show("Forogorn");
    });

    check_colors(
        r"
               4    4 - #597c95
             268    4 - #597c95
             592    4 - #597c95
             288  204 - #ffffff
             436  208 - #ffffff
             192  260 - #ff0000
             204  260 - #ff0000
             212  260 - #ff0000
             192  268 - #ff0000
             276  268 - #ff0000
             192  276 - #ff0000
             220  280 - #ff0000
             240  280 - #ff0000
             272  280 - #ffffff
             304  280 - #ffffff
             328  280 - #ffffff
             340  280 - #ff0d0d
             292  284 - #ff0000
             320  284 - #ff0d0d
             192  288 - #ff0000
             228  292 - #ff0000
             300  296 - #ffffff
             432  300 - #ffffff
             372  348 - #ffffff
             284  380 - #ffffff
             300  392 - #ffffff
             312  392 - #ffffff
             160  396 - #ffffff
             436  396 - #ffffff
               4  592 - #597c95
             236  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_alert_shown_again() -> Result<()> {
    inject_touches(
        "
            338  373  b
            338  373  e
        ",
    );

    from_main(|| {
        Alert::show("Forogorn\nSopokok\nFergel");
    });

    check_colors(
        r"
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
             156   48 - #597c95
             164  204 - #ffffff
             368  204 - #ffffff
             436  208 - #ffffff
             256  244 - #ffffff
             292  248 - #ffffff
             324  252 - #ffffff
             252  272 - #ffffff
               4  280 - #597c95
             188  280 - #ffffff
             272  280 - #ffffff
             300  280 - #ffffff
             332  280 - #ffffff
             400  288 - #ffffff
             592  288 - #597c95
             272  300 - #ffffff
             308  308 - #ffffff
             240  348 - #ffffff
             164  352 - #ffffff
             364  356 - #ffffff
             436  364 - #ffffff
             284  380 - #ffffff
             300  392 - #ffffff
             204  396 - #ffffff
              28  436 - #597c95
             404  552 - #597c95
               4  592 - #597c95
             220  592 - #597c95
             592  592 - #597c95
        ",
    )
}

impl ViewTest for AlertTestView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_alert_shown()?;
        check_alert_dismissed()?;
        check_styled_alert()?;
        check_alert_shown_again()?;

        Ok(())
    }
}
