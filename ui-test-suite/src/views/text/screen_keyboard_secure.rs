use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint, set_record_probe_count,
        system_input::{system_input, system_request, tap, type_text, wait_until},
    },
};

const PASSWORD: (f32, f32) = (540.0, 115.0);
const OUTSIDE: (f32, f32) = (300.0, 540.0);

/// A password typed with the keyboard the platform really has. The text is
/// never on screen, also not while it is typed into the system text field
/// of a phone, and the events carry bullets.
#[view]
struct ScreenKeyboardSecure {
    changes: Vec<String>,

    #[init]
    password_title: Label,
    password:       TextField,
    log:            Label,
    outside_title:  Label,
}

impl Setup for ScreenKeyboardSecure {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.password_title, "password, shows only bullets", 60.0);
        self.password.set_text_size(24).set_secure(true);
        self.password.place().t(90).lr(20).h(50);
        self.password.changed.val(move |text| self.note_change(text));

        self.log.set_text_size(18);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(170).lr(20).h(26);

        title(self.outside_title, "a tap down here ends the editing", 527.0);
    }
}

impl ScreenKeyboardSecure {
    fn note_change(mut self: Weak<Self>, text: String) {
        self.log.set_text(format!("last change event: {text}"));
        self.changes.push(text);
    }
}

impl ViewTest for ScreenKeyboardSecure {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap(PASSWORD.0, PASSWORD.1)?;
        wait_until("password is edited", move || view.password.is_editing())?;
        if system_input() {
            system_request("keyboard 1")?;
            let secure = system_request("secure")?;
            ensure!(secure == "1", "{secure} secure system fields are up, 1 expected");
        }

        type_text("abc")?;
        wait_until("password holds abc", move || view.password.text() == "abc")?;
        if system_input() {
            let shown = system_request("value")?;
            ensure!(
                !shown.contains("abc"),
                "the system field shows the password: '{shown}'"
            );
        }
        checkpoint("password is edited, 3 bullets, no letters anywhere")?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("password is not edited any more", move || {
            !view.password.is_editing()
        })?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        checkpoint("3 bullets in the field, the log shows 3 bullets")?;
        from_main(move || {
            ensure!(
                view.password.text() == "abc",
                "password holds '{}'",
                view.password.text()
            );
            let leaked: Vec<&String> =
                view.changes.iter().filter(|text| text.contains(['a', 'b', 'c'])).collect();
            ensure!(
                leaked.is_empty(),
                "a change event carried the password: {leaked:?}"
            );
            ensure!(
                view.changes.last().is_some_and(|text| text == "\u{2022}\u{2022}\u{2022}"),
                "the last change event was {:?}",
                view.changes.last()
            );
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
             396    4 - #597c95
             512    4 - #597c95
             112   68 - #000000
             200   68 - #3c5465
              40   72 - #597c95
              60   72 - #597c95
              92   72 - #597c95
             184   72 - #597c95
             200   72 - #3c5465
             228   72 - #010101
             256   72 - #49667a
              40   76 - #597c95
              60   76 - #597c95
              72   76 - #000001
              92   76 - #597c95
             100   76 - #000000
             112   76 - #000000
             140   76 - #344857
             184   76 - #597c95
             192   76 - #425b6e
             196   76 - #2b3c48
             200   76 - #3c5465
             204   76 - #597c95
             320   92 - #ffffff
             364   92 - #ffffff
             428   92 - #ffffff
             476   92 - #ffffff
             524   92 - #ffffff
             576  108 - #ffffff
              56  116 - #ffffff
             200  116 - #ffffff
             284  120 - #ffffff
              96  128 - #ffffff
             164  128 - #ffffff
              20  132 - #ffffff
             232  136 - #ffffff
             336  136 - #ffffff
             392  136 - #ffffff
             448  136 - #ffffff
             496  136 - #ffffff
             540  136 - #ffffff
              80  180 - #273742
              48  184 - #0c1013
              76  184 - #597c95
              80  184 - #273742
              96  184 - #000000
             112  184 - #597c95
             120  184 - #000000
             152  184 - #000000
             156  184 - #090c0f
             168  184 - #000001
             172  184 - #1d2830
              44  188 - #000000
             124  188 - #010102
             156  188 - #010102
             364  228 - #597c95
             468  256 - #597c95
             592  256 - #597c95
             260  264 - #597c95
              92  296 - #597c95
             364  324 - #597c95
             456  352 - #597c95
             592  356 - #597c95
             184  360 - #597c95
               4  368 - #597c95
             276  424 - #597c95
             520  424 - #597c95
              96  428 - #597c95
             384  456 - #597c95
             520  524 - #597c95
              88  536 - #000000
             196  536 - #273641
              52  540 - #0b0f12
              72  540 - #597c95
              84  540 - #597c95
              96  540 - #597c95
             116  540 - #000000
             144  540 - #334756
             172  540 - #334756
             192  540 - #597c95
             216  540 - #597c95
             232  540 - #334756
             256  540 - #597c95
             264  540 - #476377
             272  540 - #456073
              88  544 - #000000
             136  544 - #000000
             180  544 - #1e2a32
             196  544 - #273641
             220  544 - #394f5f
             224  544 - #344857
             264  544 - #476377
             272  544 - #456073
             284  544 - #2b3c48
             436  580 - #597c95
             592  592 - #597c95
";
