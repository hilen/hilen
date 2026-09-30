use std::sync::Arc;

use anyhow::{Result, ensure};
use parking_lot::Mutex;

use crate::{
    self as hilen, BugReport, BugReportStyle,
    bug_report::{BugReportData, BugReportInput, BugReportView},
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{color::Color, flat::Size},
    ui::{ModalView, UIColor, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const EMPTY_FORM: &str = r"
            592    4 - #14100d
            60   24 - #f3e6d8
            224   24 - #14100d
            320   48 - #3a2a20
            416   48 - #3a2a20
            508   48 - #3a2a20
            104   72 - #644e3e
            140  100 - #f3e6d8
            76  108 - #dcd0c3
            172  108 - #f2e5d7
            212  148 - #8f7059
            288  172 - #928f8e
            72  176 - #7f7c79
            88  176 - #bcbcbc
            172  176 - #8a8785
            200  176 - #bcbcbc
            284  176 - #241b15
            592  292 - #14100d
            148  300 - #574537
            52  304 - #8a6c56
            404  328 - #14100d
            80  456 - #b08a6e
            148  456 - #14100d
            32  480 - #4a3426
            52  520 - #a07d64
            88  520 - #342921
            176  520 - #403228
            320  520 - #aa856a
            376  520 - #a78368
            428  572 - #6b5443
            536  572 - #7a6454
            588  588 - #14100d
            ";

const FILLED_FORM: &str = r"
            592    4 - #14100d
            60   24 - #f3e6d8
            108   24 - #14100d
            224   24 - #14100d
            308   48 - #3a2a20
            396   48 - #3a2a20
            500   48 - #3a2a20
            104   72 - #644e3e
            172  104 - #f3e6d8
            76  108 - #dcd0c3
            128  108 - #f3e6d8
            44  144 - #14100d
            44  176 - #f3e6d8
            76  176 - #5e544c
            140  176 - #c3b7ab
            240  176 - #241b15
            548  284 - #14100d
            52  304 - #8a6c56
            136  304 - #42342a
            360  328 - #14100d
            88  456 - #43352a
            144  456 - #8e6f59
            32  488 - #ff8a3d
            52  520 - #a07d64
            128  520 - #4d3c30
            220  520 - #ac876b
            320  520 - #aa856a
            376  520 - #a78368
            472  560 - #ff8a3d
            588  560 - #ff8a3d
            428  572 - #6b5443
            508  588 - #ff8a3d
            ";

const fn plain(hex: &str) -> UIColor {
    UIColor::Plain(Color::hex(hex))
}

/// A black and orange app, the report screen follows its palette.
const EMBER: BugReportStyle = BugReportStyle {
    page:           plain("#14100d"),
    text:           plain("#f3e6d8"),
    muted:          plain("#b08a6e"),
    panel:          plain("#241b15"),
    line:           plain("#3a2a20"),
    field_selected: plain("#33251c"),
    accent:         plain("#ff8a3d"),
    accent_text:    plain("#14100d"),
    disabled_send:  Some((plain("#3a2a20"), plain("#7a6454"))),
    check_box:      Some((plain("#241b15"), plain("#4a3426"), plain("#ff8a3d"))),
};

/// The app sets the colors of the report screen and the email of the
/// signed in user. The screen opens in the ember palette with the email
/// filled in, and the close button draws the Lucide x in the muted color.
#[view]
struct BugReportStyled {}

impl ViewTest for BugReportStyled {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let result: Arc<Mutex<Option<Option<BugReportData>>>> = Arc::new(Mutex::new(None));
        let stored = result.clone();

        let dialog = from_main(move || {
            EMBER.apply_globally();
            BugReport::set_email("player@example.com");
            let dialog = BugReportView::prepare_modally_with_input(BugReportInput {
                screenshot_png:  Vec::new(),
                screenshot_rgba: Vec::new(),
                screenshot_size: Size::default(),
                log_bytes:       120,
                keys:            Vec::new(),
            });
            dialog.modal_event().val(move |data| *stored.lock() = Some(data));
            dialog
        });
        wait_for_next_frame();
        wait_for_next_frame();

        let email = from_main(move || dialog.form.email.text().to_string());
        ensure!(email == "player@example.com", "email field holds {email:?}");

        // Dark page, a dim brown Send while the form is empty, the x icon
        // in the top right corner.
        check_colors(EMPTY_FORM)?;

        from_main(move || {
            let mut form = dialog.form;
            form.description.set_text("The game froze on the map screen");
            form.attach_keys.set_on(true);
        });
        wait_for_next_frame();

        // Filled in, Send turns orange, the checkbox dot is orange.
        check_colors(FILLED_FORM)?;

        // The close button, 28 points wide, 16 from the right edge.
        let x = from_main(move || dialog.width()) - 30.0;
        inject_touches(format!("{x:.0} 24 b\n{x:.0} 24 e"));
        wait_for_next_frame();
        ensure!(
            matches!(*result.lock(), Some(None)),
            "the close icon did not close the screen"
        );

        Ok(())
    }
}
