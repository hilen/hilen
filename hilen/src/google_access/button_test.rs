use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    google_access::{GoogleAccess, GoogleLinkButton, GoogleSignInButton},
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, system_input::wait_until},
};

const IDLE_PROBES: &str = r"
               4    4 - #597c95
             232  180 - #202d36
             292  180 - #49657a
             336  180 - #0f1419
             376  180 - #597c95
             396  180 - #131b20
             408  200 - #747775
             200  212 - #ea4335
             300  216 - #e1e1e1
             208  220 - #4285f4
             256  220 - #9d9d9d
             296  220 - #c0c0c0
             300  220 - #e1e1e1
             340  220 - #ffffff
             376  236 - #ffffff
             416  236 - #ffffff
             224  296 - #202d36
             284  300 - #597c95
             356  320 - #747775
             416  324 - #ffffff
             200  332 - #ea4335
             204  340 - #4285f4
             208  340 - #4285f4
             248  340 - #ffffff
             296  340 - #ffffff
             360  340 - #dcdcdc
             268  432 - #000000
             272  432 - #324654
             288  432 - #476377
             332  432 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";

const FAILED_PROBES: &str = r"
               4    4 - #597c95
             592    4 - #597c95
             232  180 - #202d36
             292  180 - #49657a
             336  180 - #0f1419
             376  180 - #597c95
             396  180 - #131b20
             408  200 - #747775
             200  212 - #ea4335
             300  216 - #e1e1e1
             208  220 - #4285f4
             256  220 - #9d9d9d
             296  220 - #c0c0c0
             300  220 - #e1e1e1
             340  220 - #ffffff
             416  236 - #ffffff
             224  296 - #202d36
             348  320 - #747775
             404  324 - #ffffff
             200  332 - #ea4335
             204  340 - #4285f4
             208  340 - #4285f4
             248  340 - #ffffff
             296  340 - #ffffff
             360  340 - #dcdcdc
             196  428 - #06080a
             160  432 - #527289
             220  432 - #425c6e
             280  432 - #597c95
             348  432 - #40596b
             420  432 - #000000
             592  592 - #597c95
";

const WAITING_PROBES: &str = r"
               4    4 - #597c95
             232  180 - #202d36
             292  180 - #49657a
             336  180 - #0f1419
             396  180 - #131b20
             220  200 - #747775
             308  216 - #727272
             208  220 - #5f6368
             256  220 - #e1e1e1
             260  220 - #a9a9a9
             264  220 - #e1e1e1
             380  220 - #bec0c2
             392  220 - #ffffff
             192  228 - #ffffff
             368  236 - #ffffff
             224  296 - #202d36
             284  300 - #597c95
             368  300 - #597c95
             332  320 - #747775
             416  324 - #ffffff
             308  336 - #727272
             208  340 - #5f6368
             260  340 - #a9a9a9
             264  340 - #e1e1e1
             308  340 - #727272
             380  340 - #bec0c2
             192  348 - #ffffff
             344  356 - #ffffff
             232  432 - #597c95
             340  432 - #000000
               4  592 - #597c95
             592  592 - #597c95
";

/// The buttons are 240 by 40 in the middle of the 600 wide canvas, the sign
/// in button around y 220 and the link button around y 340.
const TAP_SIGN_IN: &str = "300 220 b\n300 220 e";
const TAP_LINK: &str = "300 340 b\n300 340 e";
const CANCEL_SIGN_IN: &str = "384 220 b\n384 220 e";
const CANCEL_LINK: &str = "384 340 b\n384 340 e";

#[view]
struct GoogleAccountButtonsLook {
    failures: Vec<String>,

    #[init]
    sign_in_name: Label,
    sign_in:      GoogleSignInButton,
    link_name:    Label,
    link:         GoogleLinkButton,
    status:       Label,
}

impl GoogleAccountButtonsLook {
    fn failed(mut self: Weak<Self>, button: &str, message: String) {
        // The whole message is too long for one line of the fixture.
        self.status
            .set_text(format!("the {button} button failed, as it must with no server"));
        self.failures.push(message);
    }
}

impl Setup for GoogleAccountButtonsLook {
    fn setup(self: Weak<Self>) {
        self.sign_in_name.set_text("sign in button, the main account").set_text_size(14);
        self.sign_in_name.place().center_x().t(168).size(320, 24);
        self.sign_in.place().center_x().t(200).size(240, 40);
        self.sign_in.failed.val(move |message| self.failed("sign in", message));

        self.link_name.set_text("link button, one more account").set_text_size(14);
        self.link_name.place().center_x().t(288).size(320, 24);
        self.link.place().center_x().t(320).size(240, 40);
        self.link.failed.val(move |message| self.failed("link", message));

        self.status.set_text("nothing tapped yet").set_text_size(14);
        self.status.place().center_x().t(420).size(560, 24);
    }
}

impl ViewTest for GoogleAccountButtonsLook {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        check_colors(IDLE_PROBES)?;

        // A real tap opens the browser of whoever runs the suite. With no
        // server the sign in fails before that, which is the path checked
        // here. The answer comes a few frames later, the buttons go through
        // the accounts layer.
        let server = from_main(|| GoogleAccess::swap_server(None));
        inject_touches(TAP_SIGN_IN);
        wait_until("the sign in button reports the missing server", move || {
            view.failures.len() == 1
        })?;
        inject_touches(TAP_LINK);
        wait_until("the link button reports the missing server", move || {
            view.failures.len() == 2
        })?;
        from_main(move || GoogleAccess::swap_server(server));

        let failures = from_main(move || view.failures.clone());
        assert!(
            failures.iter().all(|message| message.contains("no Google access server")),
            "a tap did not report the missing server: {failures:?}"
        );
        assert!(!from_main(
            move || view.sign_in.is_waiting() || view.link.is_waiting()
        ));
        check_colors(FAILED_PROBES)?;

        from_main(move || {
            view.status.set_text("both buttons wait for the browser");
            view.sign_in.show_waiting_frozen();
            view.link.show_waiting_frozen();
        });
        check_colors(WAITING_PROBES)?;

        inject_touches(CANCEL_SIGN_IN);
        inject_touches(CANCEL_LINK);
        assert!(!from_main(
            move || view.sign_in.is_waiting() || view.link.is_waiting()
        ));
        from_main(move || {
            view.status.set_text("nothing tapped yet");
        });
        check_colors(IDLE_PROBES)?;

        Ok(())
    }
}
