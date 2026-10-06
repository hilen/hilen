use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Label, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::{
        check_colors,
        system_input::{tap, type_text, wait_until},
    },
};

use super::{double_tap_row, names, sample, tap_control};

/// The search field filters the open folder by name while the user
/// types, the eye switch shows the hidden entries, and a folder with
/// nothing to show says so.
#[view]
struct FileBrowserSearch {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl FileBrowserSearch {
    fn show_count(self: Weak<Self>) {
        let hidden = if self.browser.shows_hidden() {
            "shown"
        } else {
            "not shown"
        };
        self.status.set_text(format!(
            "{} entries, hidden files {hidden}",
            self.browser.entries().len()
        ));
    }
}

impl Setup for FileBrowserSearch {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser.set_open_on_tap(false).set_source(sample());

        self.status.set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
        self.show_count();
    }
}

impl ViewTest for FileBrowserSearch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let count = move || from_main(move || view.show_count());

        let search = from_main(move || browser.control_center(FileBrowserControl::Search));
        tap(search.x, search.y)?;
        type_text("ma")?;
        // A phone starts a field with a capital letter, the filter does
        // not care.
        wait_until("the search filtered", move || browser.entries().len() == 2)?;
        assert_eq!(names(browser), ["Many", "main.rs"]);

        type_text("zz")?;
        wait_until("nothing matches", move || browser.entries().is_empty())?;
        assert_eq!(from_main(move || browser.status_text().to_string()), "No matches");

        // The tap on the switch also ends the editing, so no caret blinks
        // in the checked picture.
        from_main(move || {
            browser.set_search("ma");
        });
        tap_control(browser, FileBrowserControl::HiddenSwitch);
        count();
        assert_eq!(names(browser), ["Many", "main.rs"]);
        check_colors(CHECK_1)?;

        from_main(move || {
            browser.set_search("");
        });
        count();
        assert_eq!(names(browser).len(), 12);
        assert_eq!(names(browser)[0], ".cache");
        assert_eq!(names(browser)[7], ".env");
        check_colors(CHECK_2)?;

        tap_control(browser, FileBrowserControl::HiddenSwitch);
        count();
        assert_eq!(names(browser).len(), 10);

        double_tap_row(browser, "Empty")?;
        count();
        assert_eq!(
            from_main(move || browser.status_text().to_string()),
            "This folder is empty"
        );
        check_colors(CHECK_3)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
             292    8 - #d9d9de
             500    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
             416   24 - #1c1c1e
             420   24 - #363638
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             508   60 - #ffffff
             188   80 - #64a8f7
              52   84 - #dcdce3
             192   84 - #2f8bf5
              16   88 - #dcdce3
             192   88 - #2f8bf5
             232  116 - #676768
             524  116 - #ffffff
             188  120 - #ffffff
              28  144 - #8f8f96
              88  144 - #9a9a9c
             332  344 - #ffffff
             592  412 - #ffffff
             592  568 - #597c95
             116  576 - #000000
             160  576 - #415a6d
             192  576 - #000000
              56  580 - #000000
              64  580 - #364b5a
             108  580 - #4a677c
             112  580 - #597c95
             156  580 - #3e5769
             204  580 - #597c95
             440  592 - #597c95
";

const CHECK_2: &str = r"
             272    8 - #d9d9de
             384    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
              52   84 - #dcdce3
             192   84 - #2f8bf5
             188  140 - #64a8f7
              16  144 - #8f8f96
              88  144 - #9a9a9c
             352  160 - #ffffff
             504  176 - #ffffff
             188  200 - #64a8f7
             216  232 - #ffffff
             192  268 - #2f8bf5
             444  320 - #85858c
             268  324 - #1c1c1e
             524  324 - #d3d3d6
             180  352 - #6e6e76
               4  364 - #f4f4f6
             444  380 - #85858c
             268  416 - #adadae
             504  416 - #d8d8da
             556  416 - #d1d1d3
             200  576 - #2d3e4b
              32  580 - #161f25
              80  580 - #263540
             116  580 - #000000
             132  580 - #597c95
             212  580 - #597c95
             236  580 - #000000
             404  592 - #597c95
             592  592 - #597c95
";

const CHECK_3: &str = r"
             328    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             176   24 - #373738
             180   24 - #1d1d1f
             388   24 - #ffffff
             228   56 - #1c1c1e
             216   60 - #ffffff
             508   60 - #ffffff
              20   80 - #6e6e76
              68   84 - #8e8e90
             328  116 - #97979d
             384  120 - #cacacc
             420  120 - #c3c3c6
              16  144 - #8f8f96
              76  144 - #1c1c1e
             244  328 - #ffffff
             592  404 - #ffffff
             592  560 - #597c95
              32  576 - #597c95
             116  576 - #000000
             160  576 - #415a6d
              60  580 - #000000
              64  580 - #364b5a
             108  580 - #4a677c
             156  580 - #3e5769
             196  580 - #597c95
             216  580 - #597c95
             224  580 - #000001
             248  580 - #3e5769
             456  592 - #597c95
";
