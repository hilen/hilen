use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{
        FileBrowser, FileBrowserControl, FileBrowserMode, Label, ModifiersState, Setup, TextAlignment,
        ViewData, ViewTest, view,
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
        inject_modifiers(ModifiersState::SUPER);
        tap_row(browser, "main.rs")?;
        tap_row(browser, "readme.md")?;
        inject_modifiers(ModifiersState::empty());
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
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             516   56 - #6e6e76
              36   60 - #f4f4f6
             236   84 - #404041
             232  120 - #3a3a3c
              76  144 - #1c1c1e
             368  160 - #0a6cff
             588  164 - #0a6cff
             184  176 - #0a6cff
             504  176 - #0a6cff
             252  236 - #ffffff
             444  260 - #85858c
             524  264 - #d3d3d6
             544  264 - #d8d8da
             232  296 - #676768
             532  324 - #99999e
             276  352 - #1c1c1e
               4  356 - #f4f4f6
             212  356 - #e8e8e8
             420  356 - #ffffff
             556  356 - #d1d1d3
             108  444 - #f4f4f6
             348  512 - #d9d9de
             444  536 - #98989b
             564  536 - #c7c7ce
             212  560 - #597c95
              32  580 - #597c95
              96  580 - #000001
             128  580 - #000001
             292  592 - #597c95
";

const CHECK_3: &str = r"
             592    4 - #f4f4f6
             332    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             228   56 - #1c1c1e
             236   84 - #404041
             240   84 - #1c1c1e
              24   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             444  264 - #6e6e76
             524  264 - #d3d3d6
             240  268 - #ffffff
             588  288 - #0a6cff
             184  292 - #0a6cff
             232  296 - #aecfff
             344  308 - #0a6cff
             444  320 - #85858c
               4  356 - #f4f4f6
             268  356 - #adadae
             556  356 - #d1d1d3
             572  356 - #c7c7ca
              80  536 - #72727a
             444  536 - #98989b
             564  536 - #abcdff
             536  540 - #0a6cff
             232  560 - #597c95
              32  580 - #597c95
              96  580 - #000001
             128  580 - #000001
             336  592 - #597c95
";

const CHECK_4: &str = r"
             328    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             124   24 - #676768
             228   56 - #1c1c1e
             516   56 - #6e6e76
              68   84 - #818186
             236   84 - #404041
              24   88 - #dcdce3
             232  116 - #1c1c1e
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             504  176 - #ffffff
             424  252 - #0a6cff
             232  264 - #aecfff
             220  296 - #5297ff
             276  296 - #ffffff
             508  296 - #0a6cff
             572  296 - #68a4ff
             360  308 - #0a6cff
               4  356 - #f4f4f6
             300  452 - #ffffff
             360  512 - #d9d9de
              48  532 - #cbcbcf
             444  536 - #98989b
             564  536 - #abcdff
             536  540 - #0a6cff
             292  560 - #597c95
             112  576 - #040607
             204  576 - #000000
              32  580 - #597c95
             372  592 - #597c95
";
