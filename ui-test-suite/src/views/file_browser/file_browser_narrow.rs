use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserMode, Label, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use super::{names, picked, sample, tap_row};

/// The browser at the width of a phone, with the taps of a touch screen.
/// The toolbar takes 2 rows, the sidebar and the kind and date columns
/// are gone. A single tap opens a folder, and a tap on a file adds it to
/// the picked ones, there is no Cmd key to hold.
#[view]
struct FileBrowserNarrow {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserNarrow {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser
            .set_open_on_tap(true)
            .set_mode(FileBrowserMode::PickFiles)
            .set_source(sample());
        self.browser.chosen.val(move |paths| {
            self.status.set_text(format!("chosen: {} files", paths.len()));
        });

        self.status.set_text("single taps only").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserNarrow {
    fn canvas() -> (u32, u32) {
        (360, 600)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;

        assert!(from_main(move || browser.place_center("Disk").is_none()));
        check_colors(CHECK_1)?;

        tap_row(browser, "Photos")?;
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);

        tap_row(browser, "cat.png")?;
        tap_row(browser, "dog.jpg")?;
        assert_eq!(picked(browser), ["cat.png", "dog.jpg"]);
        assert_eq!(from_main(move || browser.choice_text().to_string()), "2 files");
        check_colors(CHECK_2)?;

        tap_row(browser, "cat.png")?;
        assert_eq!(picked(browser), ["dog.jpg"]);

        Ok(())
    }
}

const CHECK_1: &str = r"
             168    4 - #f4f4f6
             116   20 - #bcbcbd
              84   44 - #d9d9de
              20   56 - #ffffff
             340   56 - #6e6e76
             252   92 - #c5c8ce
             312   92 - #7c7c83
              28  120 - #2f8bf5
              72  120 - #404041
              68  152 - #1c1c1e
              24  176 - #64a8f7
              28  180 - #2f8bf5
             212  220 - #ffffff
              24  236 - #64a8f7
              88  272 - #ffffff
              20  276 - #2f8bf5
             320  296 - #85858c
              20  324 - #6e6e76
              68  332 - #676768
              48  360 - #e4e4e5
             304  360 - #dfdfe0
              84  392 - #e4e4e5
              20  396 - #ffffff
               4  512 - #d9d9de
             204  536 - #98989b
             324  536 - #c7c7ce
              76  576 - #000000
             128  576 - #374d5c
              60  580 - #1d2931
             120  580 - #000000
             264  592 - #597c95
             352  592 - #597c95
";

const CHECK_2: &str = r"
             264    8 - #d9d9de
             172   20 - #a3a3a4
             124   24 - #9e9ea3
             340   56 - #6e6e76
              64   92 - #1c1c1e
             252   92 - #c5c8ce
             312   92 - #7c7c83
              24  116 - #64a8f7
              60  120 - #bababa
              28  124 - #2f8bf5
             184  140 - #0a6cff
              64  152 - #69a5ff
              16  180 - #c2dbff
             288  180 - #4690ff
              68  184 - #ffffff
             296  184 - #0a6cff
             136  192 - #0a6cff
               4  360 - #ffffff
             188  368 - #ffffff
              48  532 - #cbcbcf
             192  536 - #ececf0
             288  536 - #0a6cff
             324  536 - #abcdff
              32  540 - #bebec3
              60  576 - #1d2931
             128  576 - #374d5c
              52  580 - #597c95
              76  580 - #000000
              92  580 - #597c95
             120  580 - #000000
             128  580 - #374d5c
             232  592 - #597c95
";
