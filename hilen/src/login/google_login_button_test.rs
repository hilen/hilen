use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    login::{GoogleLogin, GoogleLoginButton},
    ui::{Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const IDLE_PROBES: &str = r"
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
             232  280 - #747775
             280  280 - #747775
             316  280 - #747775
             344  280 - #747775
             368  280 - #747775
             392  280 - #747775
             416  284 - #ffffff
             192  292 - #ffffff
             200  292 - #ea4335
             300  296 - #e0e0e0
             196  300 - #ffffff
             204  300 - #4285f4
             208  300 - #4285f4
             256  300 - #989898
             268  300 - #202020
             296  300 - #c3c3c3
             300  300 - #e0e0e0
             324  300 - #ffffff
             332  300 - #ffffff
             340  300 - #ffffff
             192  308 - #ffffff
             416  312 - #ffffff
             232  316 - #ffffff
             312  316 - #ffffff
             360  316 - #ffffff
             388  316 - #ffffff
               4  592 - #597c95
             304  592 - #597c95
             592  592 - #597c95
";

const WAITING_PROBES: &str = r"
               4    4 - #597c95
             312    4 - #597c95
             592    4 - #597c95
             188  280 - #747775
             252  280 - #747775
             276  280 - #747775
             348  280 - #747775
             372  280 - #747775
             412  280 - #747775
             232  284 - #ffffff
             308  296 - #797979
             208  300 - #5f6368
             256  300 - #dedede
             260  300 - #acacac
             264  300 - #dedede
             288  300 - #1f1f1f
             308  300 - #797979
             324  300 - #232323
             328  300 - #313131
             368  300 - #ffffff
             380  300 - #c6c7c9
             392  300 - #ffffff
             192  308 - #ffffff
             232  316 - #ffffff
             276  316 - #ffffff
             300  316 - #ffffff
             344  316 - #ffffff
             416  316 - #ffffff
               4  388 - #597c95
               4  592 - #597c95
             288  592 - #597c95
             592  592 - #597c95
";

/// The button sits in the middle of the 600 by 600 canvas, 240 by 40.
const TAP_BUTTON: &str = "300 300 b\n300 300 e";
const TAP_CANCEL: &str = "384 300 b\n384 300 e";

#[view]
struct GoogleLoginButtonLook {
    failure: Option<String>,

    #[init]
    button: GoogleLoginButton,
}

impl Setup for GoogleLoginButtonLook {
    fn setup(mut self: Weak<Self>) {
        self.button.place().center().size(240, 40);
        self.button.failed.val(move |message| self.failure = Some(message));
    }
}

impl ViewTest for GoogleLoginButtonLook {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        check_colors(IDLE_PROBES)?;

        // A real tap opens the browser of whoever runs the suite. With no
        // server the login fails before that, which is the path checked here.
        // The demo sets a server at launch and runs this suite from inside.
        let server = from_main(|| GoogleLogin::swap_server(None));
        inject_touches(TAP_BUTTON);
        from_main(move || GoogleLogin::swap_server(server));

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

        Ok(())
    }
}
