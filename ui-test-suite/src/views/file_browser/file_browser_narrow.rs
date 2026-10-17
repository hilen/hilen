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
             208    8 - #d9d9de
             344    8 - #d9d9de
             124   24 - #676768
              80   56 - #e8e8eb
             312   92 - #7c7c83
              72  120 - #404041
              68  156 - #3a3a3c
             352  196 - #ffffff
             216  200 - #ffffff
              20  240 - #ffffff
              88  272 - #ffffff
              56  300 - #1c1c1e
             100  300 - #8c8c8d
             320  300 - #6e6e76
              48  332 - #e4e4e5
              68  332 - #676768
             320  356 - #85858c
             292  360 - #c2c2c5
             112  388 - #1c1c1e
              56  392 - #bcbcbd
              84  392 - #e4e4e5
             296  392 - #ffffff
               4  420 - #ffffff
             204  536 - #98989b
             324  536 - #c7c7ce
              76  576 - #000000
             128  576 - #374d5c
              60  580 - #1d2931
              76  580 - #000000
             120  580 - #000000
             264  592 - #597c95
             352  592 - #597c95
";

const CHECK_2: &str = r"
             268    8 - #d9d9de
             344    8 - #d9d9de
             116   20 - #d4d4d7
             172   20 - #a3a3a4
              20   56 - #ffffff
              64   92 - #1c1c1e
             312   92 - #7c7c83
              60  120 - #bababa
             176  140 - #0a6cff
              64  152 - #69a5ff
             292  152 - #0a6cff
             320  176 - #d8e7ff
             288  180 - #4690ff
              24  184 - #0a6cff
              68  184 - #ffffff
             220  192 - #0a6cff
               4  360 - #ffffff
             180  364 - #ffffff
              48  532 - #cbcbcf
             192  536 - #ececf0
             204  536 - #98989b
             324  536 - #abcdff
              32  540 - #bebec3
             296  540 - #0a6cff
              76  576 - #000000
             128  576 - #374d5c
              60  580 - #1d2931
              92  580 - #597c95
             120  580 - #000000
             128  580 - #374d5c
             220  592 - #597c95
             352  592 - #597c95
";
