use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen, BugReportScreenshot,
    bug_report::{BugReportData, BugReportInput, BugReportView, KeyPress},
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    ui::{ModalView, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const OPEN_PROBES: &str = r"
             592    4 - #f7f8fa
              84   20 - #17191d
              60   24 - #17191d
              84   24 - #17191d
             232   24 - #bec1c7
             324   48 - #e0e2e6
             416   48 - #e0e2e6
              92   72 - #8b9199
             160  108 - #cacacb
              44  144 - #f7f8fa
             152  144 - #5d6570
             172  176 - #cccccd
             284  176 - #ececef
              60  272 - #f7f8fa
             592  288 - #f7f8fa
             148  300 - #bcc0c5
             400  336 - #f7f8fa
              80  456 - #5f6772
             144  456 - #818790
              32  480 - #c7c7cc
              36  480 - #c7c7cc
              32  488 - #ffffff
              36  488 - #ffffff
              40  488 - #ffffff
              28  492 - #ffffff
              36  492 - #ffffff
              68  520 - #bec1c7
             208  520 - #a5aab0
             220  520 - #68707a
             336  520 - #dcdfe2
             544  568 - #a6a6aa
             428  572 - #9da2a9
";

const FILLED_PROBES: &str = r"
             236   20 - #d4d7db
              60   24 - #17191d
             108   24 - #f7f8fa
             344   48 - #e0e2e6
             452   48 - #e0e2e6
             592   48 - #e0e2e6
             116  104 - #b0b0b4
             168  108 - #c0c0c3
              44  144 - #f7f8fa
              44  176 - #17191d
              88  176 - #ececef
             136  176 - #181a1e
             168  176 - #484a4e
             200  176 - #ececef
             592  212 - #f7f8fa
             136  304 - #cacdd1
             384  304 - #f7f8fa
             544  384 - #f7f8fa
              88  456 - #c8ccd0
             144  456 - #7f858e
              32  480 - #c7c7cc
              32  488 - #0a84ff
              40  488 - #ffffff
              72  520 - #757b85
             160  520 - #8a9099
             220  520 - #616974
             320  520 - #636b75
             376  520 - #666e78
             472  560 - #3c78f0
             588  560 - #3c78f0
             428  572 - #a1a6ad
             508  588 - #3c78f0
";

const SENT_PROBES: &str = r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
";

const CLOSED_PROBES: &str = r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
";

#[view]
struct BugReportDialog {}

type SharedResult = Arc<Mutex<Option<Option<BugReportData>>>>;

fn open_dialog() -> (Weak<BugReportView>, SharedResult) {
    from_main(|| {
        let input = BugReportInput {
            screenshot: BugReportScreenshot::default(),
            log_bytes:  120,
            keys:       vec![
                KeyPress {
                    code:  "KeyS".to_string(),
                    mods:  vec!["Meta".to_string()],
                    at_ms: 1000,
                },
                KeyPress {
                    code:  "Escape".to_string(),
                    mods:  Vec::new(),
                    at_ms: 2500,
                },
            ],
        };

        let view = BugReportView::prepare_modally_with_input(input);

        let result: SharedResult = Arc::new(Mutex::new(None));
        let stored = result.clone();

        view.modal_event().val(move |data| {
            *stored.lock() = Some(data);
        });

        (view, result)
    })
}

impl ViewTest for BugReportDialog {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let (dialog, result) = open_dialog();

        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert!(!dialog.form.attach_keys.on());
            assert_eq!(
                dialog.form.keys_preview.text(),
                "+0.00s  Meta+KeyS\n+1.50s  Escape"
            );
            assert_eq!(dialog.form.counter.text(), "0 / 5 (5 more)");
        });

        check_colors(OPEN_PROBES)?;

        // Send is disabled while the form is empty, tapping it does
        // nothing and the dialog stays.
        inject_touches(
            "
            530  572  b
            530  572  e
        ",
        );

        {
            let result = result.clone();
            from_main(move || {
                assert!(result.lock().is_none());
            });
        }

        // The opt in checkbox.
        inject_touches(
            "
            34  489  b
            34  489  e
        ",
        );

        // Tab hands editing from the email field to the description,
        // scoped to the dialog's touch layer. Key injection is a desktop
        // thing, a phone types through the screen keyboard.
        #[cfg(desktop)]
        {
            use crate::{
                ui::{NamedKey, UIManager, ViewTouch},
                ui_test::inject_named_key,
            };

            from_main(move || dialog.form.email.focus());
            inject_named_key(NamedKey::Tab);
            from_main(move || {
                assert!(dialog.form.description.is_selected());
                UIManager::unselect_view();
            });
        }

        from_main(move || {
            assert!(dialog.form.attach_keys.on());
            dialog.form.email.set_text("test@example.com");
            dialog.form.description.set_text("This report is long enough to send");
        });

        wait_for_next_frame();

        from_main(move || {
            assert_eq!(dialog.form.counter.text(), "34 / 5");
        });

        check_colors(FILLED_PROBES)?;

        // Send.
        inject_touches(
            "
            530  572  b
            530  572  e
        ",
        );

        {
            let result = result.clone();
            from_main(move || {
                let result = result.lock();
                let data = result
                    .as_ref()
                    .expect("Send did not close the dialog")
                    .as_ref()
                    .expect("Send produced no report data");

                assert_eq!(data.email, "test@example.com");
                assert_eq!(data.description, "This report is long enough to send");
                assert_eq!(data.keys.as_ref().expect("Opted in keys are missing").len(), 2);
            });
        }

        check_colors(SENT_PROBES)?;

        // Cancel closes without data.
        let (_dialog, result) = open_dialog();

        wait_for_next_frame();

        inject_touches(
            "
            413  572  b
            413  572  e
        ",
        );

        from_main(move || {
            let result = result.lock();
            assert!(result.as_ref().expect("Cancel did not close the dialog").is_none());
        });

        // The close cross in the top bar does the same.
        let (_dialog, result) = open_dialog();

        wait_for_next_frame();

        inject_touches(
            "
            570  24  b
            570  24  e
        ",
        );

        from_main(move || {
            let result = result.lock();
            assert!(result.as_ref().expect("Close did not close the dialog").is_none());
        });

        check_colors(CLOSED_PROBES)?;

        Ok(())
    }
}
