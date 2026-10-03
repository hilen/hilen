use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    login::{AppleLoginButton, Login},
    ui::{Setup, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const IDLE_PROBES: &str = r"
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
             592  200 - #597c95
               4  216 - #597c95
             228  280 - #000000
             368  280 - #000000
             416  284 - #000000
             392  292 - #000000
             204  296 - #ffffff
             300  296 - #222222
             196  300 - #ffffff
             200  300 - #ffffff
             204  300 - #ffffff
             252  300 - #000000
             256  300 - #6f6f6f
             268  300 - #ffffff
             296  300 - #484848
             300  300 - #222222
             324  300 - #000000
             332  300 - #000000
             196  304 - #ffffff
             200  304 - #ffffff
             204  304 - #ffffff
             228  316 - #000000
             352  316 - #000000
             380  316 - #000000
             412  316 - #000000
             592  396 - #597c95
               4  592 - #597c95
             296  592 - #597c95
             592  592 - #597c95
            ";

const WAITING_PROBES: &str = r"
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
               8  196 - #597c95
             592  200 - #597c95
             200  280 - #000000
             232  280 - #000000
             276  280 - #000000
             348  280 - #000000
             416  284 - #000000
             180  288 - #000000
             308  296 - #a1a1a1
             208  300 - #a1a1a6
             256  300 - #222222
             260  300 - #626262
             264  300 - #222222
             288  300 - #fefefe
             308  300 - #a1a1a1
             324  300 - #ffffff
             328  300 - #efefef
             368  300 - #000000
             380  300 - #414143
             392  300 - #000000
             184  316 - #000000
             232  316 - #000000
             348  316 - #000000
             416  316 - #000000
               4  388 - #597c95
             592  396 - #597c95
               4  592 - #597c95
             304  592 - #597c95
             592  592 - #597c95
            ";

const DARK_PROBES: &str = r"
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
             592  200 - #597c95
               4  216 - #597c95
             228  280 - #ffffff
             368  280 - #ffffff
             416  284 - #ffffff
             392  292 - #ffffff
             204  296 - #000000
             300  296 - #dddddd
             196  300 - #000000
             200  300 - #000000
             204  300 - #000000
             252  300 - #ffffff
             256  300 - #909090
             268  300 - #000000
             296  300 - #b7b7b7
             300  300 - #dddddd
             324  300 - #ffffff
             332  300 - #ffffff
             196  304 - #000000
             200  304 - #000000
             204  304 - #000000
             228  316 - #ffffff
             352  316 - #ffffff
             380  316 - #ffffff
             412  316 - #ffffff
             592  396 - #597c95
               4  592 - #597c95
             296  592 - #597c95
             592  592 - #597c95
            ";

/// The button sits in the middle of the 600 by 600 canvas, 240 by 40.
const TAP_BUTTON: &str = "300 300 b\n300 300 e";
const TAP_CANCEL: &str = "384 300 b\n384 300 e";

#[view]
struct AppleLoginButtonLook {
    failure: Option<String>,

    #[init]
    button: AppleLoginButton,
}

impl Setup for AppleLoginButtonLook {
    fn setup(mut self: Weak<Self>) {
        self.button.place().center().size(240, 40);
        self.button.failed.val(move |message| self.failure = Some(message));
    }
}

impl ViewTest for AppleLoginButtonLook {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        check_colors(IDLE_PROBES)?;

        // A real tap opens the browser of whoever runs the suite. With no
        // server the login fails before that, which is the path checked here.
        // The demo sets a server at launch and runs this suite from inside.
        let server = from_main(|| Login::swap_server(None));
        inject_touches(TAP_BUTTON);
        from_main(move || Login::swap_server(server));

        let failure = from_main(move || view.failure.clone());
        assert!(
            failure.as_deref().is_some_and(|message| message.contains("no login server")),
            "the tap did not report the missing server: {failure:?}"
        );
        assert!(!from_main(move || view.button.is_waiting()));

        from_main(move || view.button.show_waiting_frozen());
        check_colors(WAITING_PROBES)?;

        inject_touches(TAP_CANCEL);
        assert!(!from_main(move || view.button.is_waiting()));
        check_colors(IDLE_PROBES)?;

        // The button turns white on a dark screen and the logo black.
        from_main(|| Theme::set_mode(ThemeMode::Dark));
        check_colors(DARK_PROBES)?;

        Ok(())
    }
}
