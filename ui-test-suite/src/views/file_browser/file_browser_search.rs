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
        // Below the status bar of a phone, which takes a tap on the toolbar.
        self.browser.place().t(40).lr(0).b(40);
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
               4    4 - #597c95
             184   48 - #d9d9de
             276   48 - #d9d9de
             348   48 - #d9d9de
             116   60 - #bcbcbd
             416   64 - #1c1c1e
             420   64 - #363638
             228   96 - #1c1c1e
             436   96 - #7c7c83
             516   96 - #6e6e76
              52  124 - #dcdce3
              68  124 - #818186
              24  128 - #dcdce3
             184  152 - #ffffff
             212  156 - #e4e4e5
             524  156 - #ffffff
              52  180 - #f4f4f6
              84  184 - #cdcdcf
              88  184 - #9a9a9c
             360  352 - #ffffff
               4  380 - #f4f4f6
             592  448 - #ffffff
             116  576 - #000000
             192  576 - #000000
              56  580 - #000000
              64  580 - #364b5a
             108  580 - #4a677c
             156  580 - #3e5769
             204  580 - #597c95
              32  584 - #1c272f
             416  592 - #597c95
             564  592 - #597c95
";

const CHECK_2: &str = r"
             288   40 - #f4f4f6
             164   48 - #d9d9de
             124   64 - #676768
             436   96 - #7c7c83
             516   96 - #6e6e76
             216  100 - #ffffff
              68  124 - #818186
             152  132 - #dcdce3
             268  156 - #bfbfbf
              88  184 - #9a9a9c
             216  188 - #787879
             504  216 - #ffffff
             216  272 - #ffffff
             420  336 - #939399
             220  364 - #1c1c1e
             268  364 - #1c1c1e
             440  364 - #818188
             524  364 - #d3d3d6
               4  372 - #f4f4f6
             212  424 - #e4e4e5
             428  424 - #dfdfe0
             268  456 - #adadae
             504  456 - #d8d8da
             556  456 - #d1d1d3
             572  456 - #c7c7ca
              32  580 - #161f25
              80  580 - #263540
             116  580 - #000000
             200  580 - #2d3e4b
             236  580 - #000000
             384  592 - #597c95
             528  592 - #597c95
";

const CHECK_3: &str = r"
             328   48 - #d9d9de
             104   56 - #d9d9de
             388   60 - #ffffff
             448   60 - #e8e8eb
             176   64 - #373738
             180   64 - #1d1d1f
             228   96 - #1c1c1e
             436   96 - #7c7c83
             516   96 - #6e6e76
             216  100 - #ffffff
              68  124 - #8e8e90
              24  128 - #f4f4f6
             328  156 - #97979d
             384  160 - #cacacc
             420  160 - #c3c3c6
              76  184 - #1c1c1e
              88  184 - #9a9a9c
             360  384 - #ffffff
             592  432 - #ffffff
              32  576 - #597c95
             116  576 - #000000
             160  576 - #415a6d
              56  580 - #000000
              64  580 - #364b5a
             108  580 - #4a677c
             156  580 - #3e5769
             196  580 - #597c95
             216  580 - #597c95
             224  580 - #000001
             248  580 - #3e5769
             592  584 - #597c95
             444  592 - #597c95
";
