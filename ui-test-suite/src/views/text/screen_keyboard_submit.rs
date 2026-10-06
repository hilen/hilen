use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint, set_record_probe_count,
        system_input::{press_return, system_input, system_request, tap, type_text, wait_until},
    },
};

const NAME: (f32, f32) = (540.0, 115.0);

/// The Return key of the keyboard the platform really has. In a single line
/// field it ends the editing and fires `submitted` with the text, and on a
/// phone the screen keyboard goes down with it.
#[view]
struct ScreenKeyboardSubmit {
    submits: Vec<String>,
    ended:   Vec<String>,

    #[init]
    name_title: Label,
    name:       TextField,
    log:        Label,
}

impl Setup for ScreenKeyboardSubmit {
    fn setup(self: Weak<Self>) {
        self.name_title.set_text("name, Return submits").set_text_size(18);
        self.name_title.set_alignment(TextAlignment::Left);
        self.name_title.place().t(60).lr(20).h(26);

        self.name.set_text_size(24);
        self.name.place().t(90).lr(20).h(50);
        self.name.submitted.val(move |text| self.note(text, true));
        self.name.editing_ended.val(move |text| self.note(text, false));

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(170).lr(20).h(80);
        self.show_log();
    }
}

impl ScreenKeyboardSubmit {
    fn note(mut self: Weak<Self>, text: String, submitted: bool) {
        if submitted {
            self.submits.push(text);
        } else {
            self.ended.push(text);
        }
        self.show_log();
    }

    fn show_log(self: Weak<Self>) {
        self.log.set_text(format!(
            "submitted: {}\nediting ended: {}",
            self.submits.join(", "),
            self.ended.join(", ")
        ));
    }
}

impl ViewTest for ScreenKeyboardSubmit {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap(NAME.0, NAME.1)?;
        wait_until("name is edited", move || view.name.is_editing())?;
        type_text("Go")?;
        wait_until("name holds Go", move || view.name.text() == "Go")?;
        checkpoint("name is edited and holds Go, nothing is submitted yet")?;
        from_main(move || {
            ensure!(
                view.submits.is_empty(),
                "submitted before Return: {:?}",
                view.submits
            );
            Ok(())
        })?;

        press_return()?;
        wait_until("Return ended the editing", move || !view.name.is_editing())?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        checkpoint("the log says submitted Go and editing ended Go")?;
        from_main(move || {
            ensure!(view.submits == ["Go"], "submitted {:?}", view.submits);
            ensure!(view.ended == ["Go"], "ended with {:?}", view.ended);
            ensure!(view.name.text() == "Go", "name holds '{}'", view.name.text());
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
             316    4 - #597c95
              76   72 - #597c95
             112   72 - #000000
             116   72 - #486478
             124   72 - #3f586a
             156   72 - #2d3f4b
             176   72 - #2f414e
             192   72 - #2a3b47
              44   76 - #19232a
              64   76 - #354a59
              68   76 - #384e5e
              92   76 - #40596c
             112   76 - #000000
             124   76 - #3f586a
             128   76 - #394f5f
             140   76 - #3d5566
             176   76 - #2f414e
             188   76 - #151d22
             192   76 - #2a3b47
             248   92 - #ffffff
             344   92 - #ffffff
             384   92 - #ffffff
             432   92 - #ffffff
             484   92 - #ffffff
             528   92 - #ffffff
             576   92 - #ffffff
              20  100 - #ffffff
             160  104 - #ffffff
             292  112 - #ffffff
             296  112 - #ffffff
              52  116 - #ffffff
             120  116 - #ffffff
             192  116 - #ffffff
             292  116 - #ffffff
             308  116 - #ffffff
             312  116 - #000000
             308  120 - #ffffff
             404  132 - #ffffff
             484  132 - #ffffff
              20  136 - #ffffff
              84  136 - #ffffff
             160  136 - #ffffff
             224  136 - #ffffff
             260  136 - #ffffff
             360  136 - #ffffff
             448  136 - #ffffff
             520  136 - #ffffff
             568  136 - #ffffff
              56  196 - #263540
             132  196 - #597c95
              60  200 - #597c95
              88  200 - #050708
              96  200 - #597c95
             100  200 - #000001
             104  200 - #000001
             108  200 - #000000
             112  200 - #597c95
             144  200 - #597c95
              40  220 - #597c95
              64  220 - #597c95
              68  220 - #000001
              72  220 - #000000
              84  220 - #597c95
              88  220 - #000000
             100  220 - #597c95
             112  220 - #000001
             120  220 - #597c95
             156  220 - #597c95
              68  224 - #000001
              72  224 - #000001
              88  224 - #000000
             100  224 - #597c95
             112  224 - #000000
             160  224 - #597c95
             592  236 - #597c95
             272  240 - #597c95
             484  264 - #597c95
             360  312 - #597c95
             128  324 - #597c95
             592  340 - #597c95
               4  348 - #597c95
             228  348 - #597c95
             452  360 - #597c95
             368  420 - #597c95
             112  424 - #597c95
             468  460 - #597c95
             592  464 - #597c95
               4  472 - #597c95
             196  488 - #597c95
             300  500 - #597c95
              92  524 - #597c95
               4  592 - #597c95
             168  592 - #597c95
             268  592 - #597c95
             412  592 - #597c95
             592  592 - #597c95
";
