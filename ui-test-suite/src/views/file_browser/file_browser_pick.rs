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
             384    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
             516   56 - #6e6e76
             188   80 - #64a8f7
             192   84 - #2f8bf5
             236   84 - #404041
             232  116 - #1c1c1e
             188  140 - #64a8f7
              16  144 - #8f8f96
              76  144 - #1c1c1e
             452  184 - #ffffff
             188  200 - #c2daff
             512  200 - #0a6cff
             588  216 - #0a6cff
             252  236 - #ffffff
             184  240 - #2f8bf5
             524  264 - #e8e8eb
             236  352 - #d2d2d7
             420  356 - #ffffff
             572  356 - #e2e2e5
              32  536 - #cdcdd0
              84  536 - #a6a6ab
             444  536 - #98989b
             564  536 - #abcdff
             536  540 - #0a6cff
              48  580 - #000000
              92  580 - #597c95
             104  580 - #000000
             108  580 - #597c95
             160  580 - #273641
             352  592 - #597c95
";

const CHECK_2: &str = r"
             580   20 - #6e6e76
             124   24 - #676768
             376   56 - #c5c8ce
             192   84 - #2f8bf5
             236   84 - #404041
               8   96 - #dcdce3
             232  120 - #3a3a3c
             188  140 - #64a8f7
              88  144 - #9a9a9c
             192  148 - #2f8bf5
             180  172 - #0a6cff
             504  176 - #0a6cff
             372  188 - #0a6cff
             188  200 - #64a8f7
             252  236 - #ffffff
             524  264 - #d3d3d6
             428  324 - #dfdfe0
               4  348 - #f4f4f6
             276  352 - #1c1c1e
             504  356 - #d8d8da
             556  356 - #d1d1d3
             184  360 - #ffffff
             328  472 - #ffffff
             444  536 - #98989b
             564  536 - #c7c7ce
              32  580 - #597c95
              64  580 - #000000
             100  580 - #0c1013
             128  580 - #000000
             148  580 - #597c95
             368  588 - #597c95
             276  592 - #597c95
";

const CHECK_3: &str = r"
             388    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
             228   56 - #1c1c1e
             192   84 - #2f8bf5
             236   84 - #404041
              16   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
             188  140 - #64a8f7
              88  144 - #9a9a9c
             192  148 - #2f8bf5
             188  200 - #64a8f7
             524  264 - #d3d3d6
             240  268 - #ffffff
             184  292 - #0a6cff
             232  296 - #aecfff
             524  296 - #0a6cff
             380  308 - #0a6cff
               4  320 - #f4f4f6
             180  356 - #6e6e76
             248  356 - #e4e4e5
             556  356 - #d1d1d3
              32  536 - #cdcdd0
             444  536 - #98989b
             564  536 - #abcdff
             112  576 - #040607
             136  576 - #1b252d
              32  580 - #597c95
              48  580 - #597c95
              96  580 - #000001
             296  592 - #597c95
";

const CHECK_4: &str = r"
             388    8 - #d9d9de
             580   20 - #6e6e76
             124   24 - #676768
             448   24 - #e8e8eb
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             192   84 - #2f8bf5
             236   84 - #404041
              16   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
              20  144 - #8f8f96
              76  144 - #1c1c1e
             184  180 - #2f8bf5
             424  252 - #0a6cff
             232  264 - #aecfff
             236  292 - #9ec5ff
             276  296 - #ffffff
             496  296 - #5197ff
             572  296 - #68a4ff
             184  300 - #0a6cff
             360  308 - #0a6cff
               4  336 - #f4f4f6
             276  448 - #ffffff
             444  536 - #98989b
              32  540 - #bebec3
             536  540 - #0a6cff
             112  576 - #040607
              48  580 - #597c95
             172  580 - #000000
             200  580 - #597c95
             328  592 - #597c95
";
