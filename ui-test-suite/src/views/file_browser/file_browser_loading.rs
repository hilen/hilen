use std::sync::Arc;

use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::MemoryFiles,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Label, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use super::{double_tap_row, names, sample, tap_control};

/// A source that answers late or fails. The list shows the wait, then
/// the rows. A failed listing shows its error with a button to try
/// again. An answer that comes after the user moved on is dropped.
#[view]
struct FileBrowserLoading {
    files: Option<Arc<MemoryFiles>>,

    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl FileBrowserLoading {
    fn files(&self) -> Arc<MemoryFiles> {
        self.files.clone().expect("the fixture made its files in setup")
    }

    fn say(&self, text: &str) {
        self.status.set_text(text);
    }
}

impl Setup for FileBrowserLoading {
    fn setup(mut self: Weak<Self>) {
        let files = sample();
        files.set_holding(true);
        self.files = Some(files.clone());

        self.browser.place().t(0).lr(0).b(40);
        self.browser.set_open_on_tap(false).set_source(files);

        self.status.set_text("the source holds its answer").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserLoading {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let message = move || from_main(move || browser.status_text().to_string());

        assert!(from_main(move || browser.is_loading()));
        assert_eq!(message(), "Loading");
        assert_eq!(names(browser), Vec::<String>::new());
        check_colors(CHECK_1)?;

        from_main(move || {
            view.files().answer_held();
            view.files().set_holding(false);
            view.say("the answer came");
        });
        assert_eq!(names(browser).len(), 10);
        assert_eq!(message(), "");

        from_main(move || {
            view.files().fail_next_listing("The machine is offline");
            view.say("the next listing fails");
        });
        double_tap_row(browser, "Photos")?;
        assert_eq!(
            from_main(move || browser.error().map(ToString::to_string)),
            Some("The machine is offline".to_string())
        );
        assert_eq!(message(), "The machine is offline");
        assert_eq!(names(browser), Vec::<String>::new());
        check_colors(CHECK_2)?;

        from_main(move || view.say("Try again was tapped"));
        tap_control(browser, FileBrowserControl::Retry);
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);
        assert!(from_main(move || browser.error().is_none()));
        check_colors(CHECK_3)?;

        // Trip is asked and still waits when the user goes back. Its
        // answer comes after the one for Photos and must not show.
        from_main(move || {
            view.files().set_holding(true);
            view.say("Trip was asked, then Back, both answers wait");
        });
        double_tap_row(browser, "Trip")?;
        tap_control(browser, FileBrowserControl::Back);
        assert!(from_main(move || browser.is_loading()));
        from_main(move || {
            view.files().answer_held();
            view.say("both answers came, the late one for Trip is dropped");
        });
        assert_eq!(from_main(move || browser.crumb_titles()), ["Disk", "Photos"]);
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
              112    8 - #d9d9de
              312    8 - #d9d9de
              388   20 - #ffffff
              124   24 - #676768
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              436   56 - #7c7c83
              516   56 - #6e6e76
               36   60 - #f4f4f6
              216   60 - #ffffff
               52   84 - #dcdce3
               24   88 - #dcdce3
              148   96 - #dcdce3
               52  112 - #f4f4f6
              360  116 - #6e6e76
              360  120 - #6e6e76
              392  120 - #818188
               76  144 - #1c1c1e
              580  292 - #ffffff
              240  340 - #ffffff
                4  356 - #f4f4f6
              592  536 - #ffffff
              256  560 - #597c95
              416  560 - #597c95
              108  576 - #000000
               32  580 - #597c95
               64  580 - #000001
              108  580 - #000000
              132  580 - #597c95
              156  580 - #0d1215
              352  592 - #597c95
              480  592 - #597c95
";

const CHECK_2: &str = r"
              248    8 - #d9d9de
              312    8 - #d9d9de
              388   20 - #ffffff
              124   24 - #9e9ea3
              168   24 - #ffffff
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              436   56 - #7c7c83
              516   56 - #6e6e76
              216   60 - #ffffff
               52   84 - #f4f4f6
               24  112 - #dcdce3
               52  112 - #dcdce3
               20  116 - #dcdce3
               60  116 - #dcdce3
              332  120 - #e56d64
              380  120 - #dd4236
              384  120 - #e57067
              436  120 - #e9857e
              152  124 - #dcdce3
               76  144 - #1c1c1e
              592  284 - #ffffff
              224  328 - #ffffff
              592  512 - #ffffff
              268  560 - #597c95
               32  580 - #597c95
               56  580 - #000000
              112  580 - #0c1013
              128  580 - #597c95
              156  580 - #151d22
              380  592 - #597c95
              500  592 - #597c95
";

const CHECK_3: &str = r"
             296    8 - #d9d9de
             400    8 - #d9d9de
             124   24 - #9e9ea3
             176   24 - #d6d6d6
             452   24 - #f9f9f9
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
              68   84 - #8e8e90
             184   84 - #ffffff
             224   84 - #bababa
             136  100 - #dcdce3
              20  112 - #dcdce3
              60  116 - #dcdce3
             228  116 - #a7a7a8
             416  116 - #ffffff
             504  116 - #b7b7bb
             556  116 - #d4d4d7
              76  144 - #1c1c1e
             232  144 - #1c1c1e
             444  144 - #6e6e76
             188  148 - #ffffff
               4  356 - #f4f4f6
             444  360 - #ffffff
             592  556 - #ffffff
             248  560 - #597c95
             476  560 - #597c95
             132  576 - #000000
              32  580 - #030506
              68  580 - #597c95
             132  580 - #000000
             364  592 - #597c95
";

const CHECK_4: &str = r"
             308    8 - #d9d9de
             400    8 - #d9d9de
             516    8 - #d9d9de
             124   24 - #9e9ea3
             176   24 - #d6d6d6
             452   24 - #f9f9f9
             228   56 - #1c1c1e
             516   56 - #6e6e76
              68   84 - #8e8e90
             184   84 - #ffffff
             224   84 - #bababa
              20  112 - #dcdce3
             228  116 - #a7a7a8
             416  116 - #ffffff
             504  116 - #b7b7bb
             556  116 - #d4d4d7
              76  144 - #1c1c1e
             232  144 - #1c1c1e
             444  144 - #6e6e76
             188  148 - #ffffff
             364  344 - #ffffff
               4  356 - #f4f4f6
             592  392 - #ffffff
             112  560 - #597c95
             488  560 - #597c95
             340  576 - #020203
              44  580 - #000000
             176  580 - #25343f
             216  580 - #030506
             272  580 - #597c95
             388  580 - #597c95
             588  592 - #597c95
";
