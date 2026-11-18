use std::sync::Arc;

use anyhow::{Result, ensure};
use parking_lot::Mutex;

use crate::{
    self as hilen, BugReport, BugReportScreenshot, BugReportStyle,
    bug_report::{BugReportData, BugReportInput, BugReportView},
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::Color,
    ui::{ModalView, UIColor, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const EMPTY_FORM: &str = r"
             592    4 - #14100d
              84   20 - #f3e6d8
              60   24 - #f3e6d8
             224   24 - #14100d
             316   48 - #3a2a20
             412   48 - #3a2a20
             508   48 - #3a2a20
              92   72 - #826651
             136  100 - #f3e6d8
              72  108 - #d7cbbe
             168  108 - #f3e6d8
             152  144 - #b08a6e
             196  172 - #bcbcbc
             284  172 - #8c8988
              84  176 - #bbbbbb
             148  176 - #bcbcbc
             168  176 - #8a8785
             280  176 - #241b15
              60  272 - #14100d
             592  288 - #14100d
             400  336 - #14100d
              80  456 - #ae886c
             144  456 - #8c6e58
              32  480 - #4a3426
              36  480 - #4a3426
              52  520 - #a48167
             132  520 - #14100d
             208  520 - #675141
             320  520 - #ab866b
             544  568 - #655143
             428  572 - #6f5746
             544  572 - #7a6454
            ";

const FILLED_FORM: &str = r"
             592    4 - #14100d
              60   24 - #f3e6d8
             224   24 - #14100d
             308   48 - #3a2a20
             392   48 - #3a2a20
             496   48 - #3a2a20
             128   72 - #7e634f
             168  104 - #f3e6d8
              72  108 - #d7cbbe
             124  108 - #f2e5d7
              72  144 - #91725b
             152  144 - #b08a6e
              40  172 - #f3e6d8
             136  176 - #c6baae
             236  176 - #241b15
             128  180 - #ded1c4
              68  268 - #6d5544
             544  284 - #14100d
             356  328 - #14100d
              80  456 - #ae886c
             128  456 - #14100d
              32  480 - #4a3426
              36  480 - #4a3426
              32  488 - #ff8a3d
              52  520 - #a48167
             132  520 - #14100d
             208  520 - #675141
             320  520 - #ab866b
             472  560 - #ff8a3d
             588  560 - #ff8a3d
             428  572 - #6f5746
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
                screenshot: BugReportScreenshot::default(),
                log_bytes:  120,
                keys:       Vec::new(),
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
            let form = dialog.form;
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
