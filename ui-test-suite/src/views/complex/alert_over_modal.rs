use anyhow::Result;
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Alert, GREEN, Label, ModalView, Setup, Size, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_touches},
};

/// A dialog with loud content across its center, where the alert panel
/// lands. Stacked modals used to share one z position, so the alert
/// interleaved with the dialog under it: the alert text won the depth
/// test while its panel lost, and both texts showed through each other.
#[view]
struct BusyDialog {
    event: OnceEvent,

    #[init]
    title: Label,
}

impl Setup for BusyDialog {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(16);

        self.title.set_text("Busy busy busy dialog");
        self.title.set_text_color(GREEN);
        self.title.set_text_size(40);
        self.title.set_multiline(true);
        self.title.place().lrt(12).h(360);
    }
}

impl ModalView for BusyDialog {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (400, 400).into()
    }
}

#[view]
struct AlertOverModal {}

fn tap_ok(alert: Weak<Alert>) {
    let frame = from_main(move || *alert.frame());
    let x = frame.center().x;
    let y = frame.max_y() - 22.0;
    inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
    wait_for_next_frame();
}

fn dialog_block() -> &'static str {
    r"
       4    4 - #597c95
     260    8 - #597c95
     132  104 - #ffffff
     496  108 - #ffffff
     352  116 - #ffffff
     356  256 - #00ff00
     172  260 - #ffffff
     284  264 - #55ff55
     328  264 - #01ff01
     244  268 - #01ff01
     284  268 - #55ff55
     368  268 - #ffffff
     432  268 - #01ff01
     284  272 - #55ff55
     308  272 - #ffffff
     192  276 - #00ff00
     220  276 - #ffffff
     392  276 - #00ff00
     264  280 - #02ff02
     356  280 - #01ff01
     328  288 - #00ff00
     288  308 - #00ff00
     296  316 - #69ff69
     324  316 - #ffffff
     252  320 - #ffffff
     296  320 - #69ff69
     348  332 - #01ff01
       4  384 - #597c95
     188  496 - #ffffff
     432  496 - #ffffff
       4  592 - #597c95
     592  592 - #597c95
    "
}

fn check_alert_covers_dialog() -> Result<Weak<Alert>> {
    let alert = from_main(|| Alert::prepare_modally_with_input("Covered".to_string()));
    wait_for_next_frame();

    check_colors(
        r"
           4    4 - #435d70
         592    4 - #435d70
         120  100 - #bfbfbf
         408  100 - #bfbfbf
         264  128 - #bfbfbf
         488  172 - #bfbfbf
         592  200 - #435d70
         180  248 - #f9f9f9
         232  248 - #f9f9f9
         404  248 - #f9f9f9
           4  272 - #435d70
         276  276 - #f9f9f9
         280  276 - #f9f9f9
         304  280 - #f9f9f9
         308  280 - #a7a7a7
         316  280 - #f9f9f9
         404  300 - #f9f9f9
         228  320 - #f9f9f9
         296  328 - #f9f9f9
         292  332 - #f9f9f9
         296  332 - #f9f9f9
         364  336 - #f9f9f9
         168  344 - #f9f9f9
         432  344 - #f9f9f9
         592  372 - #435d70
           4  420 - #435d70
         232  444 - #bfbfbf
         344  480 - #bfbfbf
         128  496 - #bfbfbf
         488  496 - #bfbfbf
           4  592 - #435d70
         256  592 - #435d70
        ",
    )?;

    Ok(alert)
}

impl ViewTest for AlertOverModal {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let dialog = from_main(BusyDialog::prepare_modally);
        wait_for_next_frame();

        check_colors(dialog_block())?;

        let alert = check_alert_covers_dialog()?;

        tap_ok(alert);

        check_colors(dialog_block())?;

        from_main(move || dialog.hide_modal(()));
        wait_for_next_frame();

        Ok(())
    }
}
