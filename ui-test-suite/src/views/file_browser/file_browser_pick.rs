use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{
        FileBrowser, FileBrowserControl, FileBrowserMode, Label, Modifiers, Setup, TextAlignment, ViewData,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_modifiers},
};

use super::{double_tap_row, names, picked, sample, tap_control, tap_row};

/// The 3 pick modes and the bar at the bottom. A folder pick gives the
/// picked folder or the open one, a file pick needs a picked file, a
/// pick of several counts them and shows only the wanted extensions.
#[view]
struct FileBrowserPick {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserPick {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser
            .set_open_on_tap(false)
            .set_mode(FileBrowserMode::PickFolder)
            .set_source(sample());
        self.browser.chosen.val(move |paths| {
            let texts: Vec<String> = paths.iter().map(FilePath::slash_text).collect();
            self.status.set_text(format!("chosen: {}", texts.join(", ")));
        });
        self.browser.cancelled.sub(move || {
            self.status.set_text("cancelled");
        });

        self.status.set_text("nothing chosen yet").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserPick {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let choice = move || from_main(move || browser.choice_text().to_string());
        let status = move || from_main(move || view.status.text().to_string());

        // A folder pick: with nothing picked the open folder is the choice.
        assert_eq!(choice(), "Disk");
        tap_row(browser, "Photos")?;
        assert_eq!(choice(), "Disk/Photos");
        // A file is shown faint and cannot be picked.
        tap_row(browser, "main.rs")?;
        assert_eq!(picked(browser), ["Photos"]);
        check_colors(CHECK_1)?;
        tap_control(browser, FileBrowserControl::Choose);
        assert_eq!(status(), "chosen: Disk/Photos");

        // A file pick: Choose is off until a file is picked.
        from_main(move || {
            browser.set_mode(FileBrowserMode::PickFile);
        });
        assert_eq!(choice(), "");
        tap_control(browser, FileBrowserControl::Choose);
        assert_eq!(status(), "chosen: Disk/Photos");
        tap_row(browser, "Music")?;
        assert_eq!(choice(), "");
        check_colors(CHECK_2)?;
        tap_row(browser, "main.rs")?;
        assert_eq!(choice(), "Disk/main.rs");
        check_colors(CHECK_3)?;
        // A double tap on a file is pick and Choose in one.
        double_tap_row(browser, "readme.md")?;
        assert_eq!(status(), "chosen: Disk/readme.md");

        // A pick of several, only code and documents.
        from_main(move || {
            browser.set_mode(FileBrowserMode::PickFiles).set_extensions(&["rs", "MD"]);
        });
        assert_eq!(names(browser).len(), 8);
        assert_eq!(names(browser)[6..], ["main.rs", "readme.md"]);
        inject_modifiers(Modifiers::META);
        tap_row(browser, "main.rs")?;
        tap_row(browser, "readme.md")?;
        inject_modifiers(Modifiers::empty());
        assert_eq!(choice(), "2 files");
        check_colors(CHECK_4)?;
        tap_control(browser, FileBrowserControl::Choose);
        assert_eq!(status(), "chosen: Disk/main.rs, Disk/readme.md");

        tap_control(browser, FileBrowserControl::Cancel);
        assert_eq!(status(), "cancelled");

        Ok(())
    }
}

const CHECK_1: &str = r"
             328    8 - #d9d9de
             124   24 - #676768
             436   56 - #7c7c83
             516   56 - #6e6e76
             236   84 - #404041
              24   88 - #dcdce3
             232  116 - #1c1c1e
              76  144 - #1c1c1e
             356  192 - #0a6cff
             536  192 - #0a6cff
             184  204 - #0a6cff
             252  236 - #ffffff
             444  260 - #c0c0c7
             232  296 - #cdcdd2
             516  296 - #ffffff
               4  336 - #f4f4f6
             236  352 - #d2d2d7
             420  356 - #ffffff
             572  356 - #e2e2e5
             324  460 - #ffffff
             228  512 - #d9d9de
              32  536 - #cdcdd0
              84  536 - #a6a6ab
             444  536 - #98989b
             564  536 - #abcdff
             536  540 - #0a6cff
              48  576 - #000000
              40  580 - #597c95
             104  580 - #000000
             160  580 - #273641
             280  592 - #597c95
             372  592 - #597c95
";

const CHECK_2: &str = r"
              360    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
              436   56 - #7c7c83
              516   56 - #6e6e76
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              368  160 - #0a6cff
              588  164 - #0a6cff
              184  176 - #0a6cff
              504  176 - #0a6cff
              252  236 - #ffffff
              496  260 - #85858c
              220  264 - #1c1c1e
              268  264 - #1c1c1e
              424  264 - #ffffff
              536  264 - #6e6e76
              532  324 - #99999e
                4  356 - #f4f4f6
              248  356 - #e4e4e5
              420  356 - #ffffff
              400  432 - #ffffff
              244  512 - #d9d9de
              564  536 - #c7c7ce
               32  580 - #597c95
               64  580 - #000000
               96  580 - #000001
              128  580 - #000001
              348  592 - #597c95
              464  592 - #597c95
";

const CHECK_3: &str = r"
              320    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              516   56 - #6e6e76
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              184  176 - #ffffff
              504  176 - #ffffff
              232  236 - #d8d8d8
              444  260 - #85858c
              508  260 - #85858c
              240  268 - #ffffff
              184  292 - #0a6cff
              232  296 - #aecfff
              308  304 - #0a6cff
              384  308 - #0a6cff
              532  324 - #99999e
                4  348 - #f4f4f6
              248  356 - #e4e4e5
              408  460 - #ffffff
              236  512 - #d9d9de
               40  536 - #9a9aa0
              564  536 - #abcdff
              536  540 - #0a6cff
               32  580 - #597c95
               96  580 - #000001
              128  580 - #000001
              336  592 - #597c95
              444  592 - #597c95
";

const CHECK_4: &str = r"
              116    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
              436   56 - #7c7c83
              516   56 - #6e6e76
              236   84 - #404041
               24   88 - #dcdce3
              232  116 - #1c1c1e
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              184  176 - #ffffff
              504  176 - #ffffff
              232  236 - #d8d8d8
              184  260 - #0a6cff
              444  260 - #d8e7ff
              232  264 - #aecfff
              236  292 - #9ec5ff
              276  296 - #ffffff
              508  296 - #0a6cff
              588  304 - #0a6cff
              368  308 - #0a6cff
                4  356 - #f4f4f6
              396  424 - #ffffff
              296  512 - #d9d9de
              536  540 - #0a6cff
               92  576 - #597c95
              204  576 - #000000
               32  580 - #597c95
               64  580 - #000000
              172  580 - #000000
              372  592 - #597c95
              464  592 - #597c95
";
