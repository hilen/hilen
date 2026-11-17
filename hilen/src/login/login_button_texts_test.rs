use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    login::{AppleLoginButton, GoogleLoginButton},
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const IDLE_PROBES: &str = r"
               4    4 - #597c95
             260  212 - #445e71
             376  212 - #000000
             308  216 - #2a3b47
             228  220 - #597c95
             180  236 - #747775
             196  244 - #ea4335
             204  248 - #a1c2fa
             208  252 - #4285f4
             240  252 - #ffffff
             268  252 - #a0a0a0
             292  252 - #fcfcfc
             200  256 - #34a853
             204  256 - #34a853
             348  260 - #ffffff
             180  264 - #747775
             412  268 - #ffffff
             224  316 - #25333d
             248  316 - #597c95
             304  316 - #3c5465
             360  316 - #597c95
             240  352 - #000000
             264  352 - #c6c6c6
             268  352 - #6c6c6c
             292  352 - #030303
             324  352 - #313131
             380  352 - #000000
             200  356 - #ffffff
             344  368 - #000000
             416  368 - #000000
               4  592 - #597c95
             592  592 - #597c95
";

const WAITING_PROBES: &str = r"
             592    4 - #597c95
             260  212 - #445e71
             376  212 - #000000
             288  216 - #597c95
             308  216 - #2a3b47
             376  216 - #000000
             228  220 - #597c95
             180  236 - #747775
             208  248 - #5f6368
             364  248 - #ffffff
             180  252 - #747775
             272  252 - #b0b0b0
             372  252 - #919497
             384  252 - #97999d
             180  264 - #747775
             188  268 - #ffffff
             232  268 - #ffffff
             320  268 - #ffffff
             256  312 - #324654
             224  316 - #25333d
             248  316 - #597c95
             304  316 - #3c5465
             360  316 - #597c95
             400  332 - #000000
             208  348 - #a1a1a6
             364  348 - #000000
             272  352 - #5a5a5a
             196  356 - #a1a1a6
             320  368 - #000000
             416  368 - #000000
               4  592 - #597c95
             592  592 - #597c95
";

/// Cancel of the Google button, which is 240 by 40 with its top at 230.
const TAP_GOOGLE_CANCEL: &str = "384 250 b\n384 250 e";
/// Cancel of the Apple button, with its top at 330.
const TAP_APPLE_CANCEL: &str = "384 350 b\n384 350 e";

#[view]
struct LoginButtonTexts {
    #[init]
    google_role: Label,
    google:      GoogleLoginButton,
    apple_role:  Label,
    apple:       AppleLoginButton,
}

impl Setup for LoginButtonTexts {
    fn setup(mut self: Weak<Self>) {
        self.google_role.set_text("google button, russian texts").set_text_size(14);
        self.google_role.place().center_x().t(204).size(240, 20);

        self.google.place().center_x().t(230).size(240, 40);
        self.google
            .set_sign_in_text("Войти через Google")
            .set_waiting_text("Ждём вход")
            .set_cancel_text("Отмена");

        self.apple_role.set_text("apple button, russian texts").set_text_size(14);
        self.apple_role.place().center_x().t(304).size(240, 20);

        self.apple.place().center_x().t(330).size(240, 40);
        self.apple
            .set_sign_in_text("Войти через Apple")
            .set_waiting_text("Ждём вход")
            .set_cancel_text("Отмена");
    }
}

impl ViewTest for LoginButtonTexts {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        check_colors(IDLE_PROBES)?;

        from_main(move || {
            view.google.show_waiting_frozen();
            view.apple.show_waiting_frozen();
        });
        check_colors(WAITING_PROBES)?;

        // The way back to the rest look has to take the text of the app too,
        // not the English one.
        inject_touches(TAP_GOOGLE_CANCEL);
        inject_touches(TAP_APPLE_CANCEL);
        assert!(!from_main(move || view.google.is_waiting()));
        assert!(!from_main(move || view.apple.is_waiting()));
        check_colors(IDLE_PROBES)?;

        Ok(())
    }
}
