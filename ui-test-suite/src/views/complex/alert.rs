use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Alert, ModalView, RED, TextAlignment, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct AlertTestView {}

fn show_alert(message: &str) -> Weak<Alert> {
    let message = message.to_string();
    let alert = from_main(|| Alert::prepare_modally_with_input(message));
    wait_for_next_frame();
    alert
}

/// The alert sizes itself to its message, so the tap point comes from
/// its frame instead of hardcoded coordinates. The OK button is the
/// bottom row of the alert.
fn tap_ok(alert: Weak<Alert>) {
    let frame = from_main(move || *alert.frame());
    let x = frame.center().x;
    let y = frame.max_y() - 22.0;
    inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
    wait_for_next_frame();
}

fn check_alert_shown() -> Result<Weak<Alert>> {
    let alert = show_alert("Forogorn\nSopokok\nFergel");

    check_colors(
        r"
           4    4 - #435d70
         312    4 - #435d70
         592    4 - #435d70
         172  236 - #f9f9f9
         432  240 - #f9f9f9
         296  260 - #f9f9f9
         312  260 - #f9f9f9
         328  260 - #1c1c1e
         312  272 - #f9f9f9
         276  276 - #f9f9f9
         292  280 - #f9f9f9
         300  280 - #f9f9f9
         312  280 - #f9f9f9
           4  292 - #435d70
         280  292 - #b9b9b9
         280  296 - #b9b9b9
         292  296 - #202022
         296  296 - #a0a0a1
         304  296 - #f9f9f9
         312  296 - #202022
         208  300 - #f9f9f9
         388  300 - #f9f9f9
         592  308 - #435d70
         296  348 - #f9f9f9
         168  360 - #f9f9f9
         232  364 - #f9f9f9
         360  364 - #f9f9f9
         428  364 - #f9f9f9
         424  540 - #435d70
           4  592 - #435d70
         256  592 - #435d70
         592  592 - #435d70
        ",
    )?;

    Ok(alert)
}

fn check_alert_dismissed(alert: Weak<Alert>) -> Result<()> {
    tap_ok(alert);

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

fn check_styled_alert() -> Result<Weak<Alert>> {
    let styled = from_main(|| {
        Alert::with_label(|l| {
            l.set_text_color(RED).set_text_size(50).set_alignment(TextAlignment::Left);
        });
        Alert::prepare_modally_with_input("Forogorn".to_string())
    });
    wait_for_next_frame();

    check_colors(
        r"
           4    4 - #435d70
         284    4 - #435d70
         592    4 - #435d70
         428  232 - #f9f9f9
         208  260 - #ff0000
         324  268 - #fba8a8
         364  268 - #fba8a8
         380  268 - #fba8a8
         292  272 - #ff0000
         308  272 - #ff0101
         220  276 - #ff0000
         240  280 - #f9f9f9
         264  280 - #fd6565
         340  280 - #f9f9f9
         264  284 - #fd6565
         320  284 - #f9f9f9
         264  288 - #fd6565
         308  288 - #ff0101
         364  288 - #ff0000
         208  292 - #ff0000
         264  292 - #fd6565
         288  292 - #ff0101
         348  292 - #ff0000
         396  292 - #ff0000
         316  300 - #f9f9f9
         292  348 - #f9f9f9
         168  364 - #f9f9f9
         232  368 - #f9f9f9
         356  368 - #f9f9f9
         424  368 - #f9f9f9
           4  592 - #435d70
         592  592 - #435d70
        ",
    )?;

    Ok(styled)
}

fn check_alert_shown_again() -> Result<Weak<Alert>> {
    let again = show_alert("Forogorn\nSopokok\nFergel");

    check_colors(
        r"
           4    4 - #435d70
         312    4 - #435d70
         592    4 - #435d70
         172  236 - #f9f9f9
         432  240 - #f9f9f9
         296  260 - #f9f9f9
         312  260 - #f9f9f9
         328  260 - #1c1c1e
         312  272 - #f9f9f9
         276  276 - #f9f9f9
         292  280 - #f9f9f9
         300  280 - #f9f9f9
         312  280 - #f9f9f9
           4  292 - #435d70
         280  292 - #b9b9b9
         280  296 - #b9b9b9
         292  296 - #202022
         296  296 - #a0a0a1
         304  296 - #f9f9f9
         312  296 - #202022
         208  300 - #f9f9f9
         388  300 - #f9f9f9
         592  308 - #435d70
         296  348 - #f9f9f9
         168  360 - #f9f9f9
         232  364 - #f9f9f9
         360  364 - #f9f9f9
         428  364 - #f9f9f9
         424  540 - #435d70
           4  592 - #435d70
         256  592 - #435d70
         592  592 - #435d70
        ",
    )?;

    Ok(again)
}

impl ViewTest for AlertTestView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let alert = check_alert_shown()?;
        check_alert_dismissed(alert)?;

        let styled = check_styled_alert()?;
        tap_ok(styled);

        let again = check_alert_shown_again()?;
        tap_ok(again);

        Ok(())
    }
}
