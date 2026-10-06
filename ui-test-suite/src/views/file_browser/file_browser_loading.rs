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
             332    8 - #d9d9de
             580   20 - #6e6e76
             124   24 - #676768
             424   24 - #c3c3c9
             452   24 - #f9f9f9
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             436   56 - #7c7c83
             216   60 - #ffffff
             508   60 - #ffffff
              24   84 - #dcdce3
              52   84 - #dcdce3
              16   88 - #dcdce3
             148   96 - #dcdce3
             388  120 - #c4c4c7
             404  120 - #d2d2d5
              16  144 - #8f8f96
              28  144 - #8f8f96
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             260  344 - #ffffff
             592  520 - #ffffff
             108  576 - #000000
             136  576 - #2e404d
              32  580 - #597c95
              68  580 - #597c95
              92  580 - #597c95
             108  580 - #000000
             152  580 - #263540
             156  580 - #0d1215
             380  592 - #597c95
             500  592 - #597c95
";

const CHECK_2: &str = r"
             296    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #9e9ea3
             164   24 - #9f9fa0
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             436   56 - #7c7c83
             216   60 - #ffffff
             508   60 - #ffffff
              52   84 - #f4f4f6
              24  112 - #dcdce3
             328  120 - #ed9d97
             332  120 - #e56d64
             344  120 - #efa8a2
             380  120 - #dd4236
             384  120 - #e57067
             432  120 - #e0564c
             436  120 - #e9857e
              16  144 - #8f8f96
              76  144 - #1c1c1e
             400  160 - #b4b4b7
             592  312 - #ffffff
             592  556 - #ffffff
             156  576 - #151d22
              32  580 - #597c95
              56  580 - #000000
             108  580 - #54758c
             128  580 - #597c95
             352  592 - #597c95
             476  592 - #597c95
";

const CHECK_3: &str = r"
             332    8 - #d9d9de
             500    8 - #d9d9de
             172   20 - #a3a3a4
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #9e9ea3
             388   24 - #ffffff
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
             188   80 - #64a8f7
              52   84 - #f4f4f6
             192   84 - #2f8bf5
             224   84 - #bababa
             192   88 - #2f8bf5
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
             504  140 - #85858c
              16  144 - #8f8f96
              76  144 - #1c1c1e
              84  144 - #cdcdcf
             232  144 - #1c1c1e
             188  148 - #ffffff
             344  356 - #ffffff
             592  552 - #ffffff
             268  560 - #597c95
              32  580 - #030506
              64  580 - #000001
             148  580 - #597c95
             156  580 - #597c95
             376  592 - #597c95
";

const CHECK_4: &str = r"
             332    8 - #d9d9de
             500    8 - #d9d9de
             172   20 - #a3a3a4
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #9e9ea3
             388   24 - #ffffff
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
             188   80 - #64a8f7
              52   84 - #f4f4f6
             192   84 - #2f8bf5
             224   84 - #bababa
             192   88 - #2f8bf5
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
             504  140 - #85858c
              16  144 - #8f8f96
              84  144 - #cdcdcf
             232  144 - #1c1c1e
             188  148 - #ffffff
             348  348 - #ffffff
             592  392 - #ffffff
             488  560 - #597c95
             340  576 - #020203
              44  580 - #000000
             176  580 - #25343f
             272  580 - #597c95
             388  580 - #597c95
             588  592 - #597c95
";
