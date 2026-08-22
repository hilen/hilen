use anyhow::Result;
use hilen::{
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
             252  236 - #dadada
             276  244 - #989898
             312  244 - #000000
             352  244 - #000000
             292  248 - #ffffff
             248  252 - #8b8b8b
             276  252 - #989898
             328  252 - #000000
             280  276 - #313131
             344  276 - #676767
             256  280 - #010101
             304  284 - #ffffff
             280  288 - #313131
             280  292 - #313131
             268  304 - #5d5d5d
             272  304 - #5d5d5d
             336  304 - #cfcfcf
             308  312 - #ffffff
             336  312 - #cfcfcf
             264  320 - #000000
             296  320 - #d4d4d4
             336  320 - #cfcfcf
             288  376 - #0101e7
             296  380 - #0101e7
             304  380 - #0101e7
             160  384 - #ffffff
             440  388 - #ffffff
             304  392 - #0101e7
             288  396 - #0000e7
               4  592 - #597c95
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
             332    4 - #597c95
             592    4 - #597c95
             164  204 - #ffffff
             436  204 - #ffffff
             208  264 - #ff0000
             252  272 - #ff0000
             308  272 - #ff0101
             356  272 - #ff0000
             232  280 - #ffffff
             332  280 - #ffffff
             384  280 - #ffa3a3
             276  284 - #ffffff
             384  284 - #ffa3a3
             384  288 - #ffa3a3
             384  292 - #ffa3a3
             192  296 - #ff0000
             228  296 - #ff0101
             300  296 - #ff0101
             348  296 - #ff0000
             288  376 - #0101e7
             296  380 - #0101e7
             304  380 - #0101e7
             304  384 - #0101e7
             304  388 - #0101e7
             296  392 - #0101e7
             304  392 - #0101e7
             292  396 - #0000e7
             312  396 - #ffffff
             164  400 - #ffffff
             436  400 - #ffffff
               4  592 - #597c95
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
             252  236 - #dadada
             276  244 - #989898
             312  244 - #000000
             352  244 - #000000
             292  248 - #ffffff
             248  252 - #8b8b8b
             276  252 - #989898
             328  252 - #000000
             280  276 - #313131
             344  276 - #676767
             256  280 - #010101
             304  284 - #ffffff
             280  288 - #313131
             280  292 - #313131
             268  304 - #5d5d5d
             272  304 - #5d5d5d
             336  304 - #cfcfcf
             308  312 - #ffffff
             336  312 - #cfcfcf
             264  320 - #000000
             296  320 - #d4d4d4
             336  320 - #cfcfcf
             288  376 - #0101e7
             296  380 - #0101e7
             304  380 - #0101e7
             160  384 - #ffffff
             440  388 - #ffffff
             304  392 - #0101e7
             288  396 - #0000e7
               4  592 - #597c95
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
