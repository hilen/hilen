use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    login::{DiscordLoginButton, Login},
    ui::{Setup, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const IDLE_PROBES: &str = r"
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
               8  196 - #597c95
             592  212 - #597c95
             228  280 - #5865f2
             284  280 - #5865f2
             372  280 - #5865f2
             396  280 - #5865f2
             180  288 - #5865f2
             416  292 - #5865f2
             196  296 - #ffffff
             200  296 - #ffffff
             204  296 - #ffffff
             356  296 - #b8bdf9
             252  300 - #5865f2
             256  300 - #a1a8f8
             268  300 - #ffffff
             296  300 - #8790f6
             312  300 - #d4d7fc
             316  300 - #5865f2
             336  300 - #5865f2
             384  312 - #5865f2
             184  316 - #5865f2
             228  316 - #5865f2
             416  316 - #5865f2
               4  388 - #597c95
             584  404 - #597c95
             152  492 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ";

const WAITING_PROBES: &str = r"
               4    4 - #597c95
             304    4 - #597c95
             592    4 - #597c95
             156  100 - #597c95
               8  196 - #597c95
             588  196 - #597c95
             200  280 - #5865f2
             236  280 - #5865f2
             276  280 - #5865f2
             348  280 - #5865f2
             392  280 - #5865f2
             416  284 - #5865f2
             180  288 - #5865f2
             308  296 - #c1c6fa
             208  300 - #e0e3ff
             260  300 - #98a0f7
             288  300 - #fefeff
             308  300 - #c1c6fa
             324  300 - #ffffff
             328  300 - #f5f5fe
             368  300 - #5865f2
             380  300 - #8e98f7
             184  316 - #5865f2
             236  316 - #5865f2
             348  316 - #5865f2
             408  316 - #5865f2
               4  388 - #597c95
             592  392 - #597c95
             152  492 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ";

const DARK_PROBES: &str = r"
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
               8  196 - #597c95
             592  212 - #597c95
             228  280 - #5865f2
             284  280 - #5865f2
             372  280 - #5865f2
             396  280 - #5865f2
             180  288 - #5865f2
             416  292 - #5865f2
             196  296 - #ffffff
             200  296 - #ffffff
             204  296 - #ffffff
             356  296 - #b8bdf9
             252  300 - #5865f2
             256  300 - #a1a8f8
             268  300 - #ffffff
             296  300 - #8790f6
             312  300 - #d4d7fc
             316  300 - #5865f2
             336  300 - #5865f2
             384  312 - #5865f2
             184  316 - #5865f2
             228  316 - #5865f2
             416  316 - #5865f2
               4  388 - #597c95
             584  404 - #597c95
             152  492 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ";

/// The button sits in the middle of the 600 by 600 canvas, 240 by 40.
const TAP_BUTTON: &str = "300 300 b\n300 300 e";
const TAP_CANCEL: &str = "384 300 b\n384 300 e";

#[view]
struct DiscordLoginButtonLook {
    failure: Option<String>,

    #[init]
    button: DiscordLoginButton,
}

impl Setup for DiscordLoginButtonLook {
    fn setup(mut self: Weak<Self>) {
        self.button.place().center().size(240, 40);
        self.button.failed.val(move |message| self.failure = Some(message));
    }
}

impl ViewTest for DiscordLoginButtonLook {
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

        // The Discord button looks the same on a dark screen.
        from_main(|| Theme::set_mode(ThemeMode::Dark));
        check_colors(DARK_PROBES)?;

        Ok(())
    }
}
