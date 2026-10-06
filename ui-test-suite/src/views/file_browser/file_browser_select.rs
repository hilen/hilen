use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{FileBrowser, Label, ModifiersState, Point, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_modifiers},
};

use super::{picked, sample, tap_at, tap_row};

/// Picking with the pointer: a tap picks one entry, Cmd or Ctrl adds and
/// removes, Shift takes the range, a tap under the last row picks
/// nothing.
#[view]
struct FileBrowserSelect {
    changes: usize,

    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserSelect {
    fn setup(mut self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser.set_open_on_tap(false).set_source(sample());
        self.browser.selection_changed.val(move |paths| {
            self.changes += 1;
            let names: Vec<&str> = paths.iter().map(FilePath::name).collect();
            self.status.set_text(format!("picked: {}", names.join(", ")));
        });

        self.status.set_text("picked:").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserSelect {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;

        tap_row(browser, "main.rs")?;
        assert_eq!(picked(browser), ["main.rs"]);
        tap_row(browser, "readme.md")?;
        assert_eq!(picked(browser), ["readme.md"]);
        check_colors(CHECK_1)?;

        inject_modifiers(ModifiersState::SUPER);
        tap_row(browser, "archive.zip")?;
        assert_eq!(picked(browser), ["archive.zip", "readme.md"]);
        tap_row(browser, "Music")?;
        assert_eq!(picked(browser), ["Music", "archive.zip", "readme.md"]);
        check_colors(CHECK_2)?;
        tap_row(browser, "readme.md")?;
        assert_eq!(picked(browser), ["Music", "archive.zip"]);

        // The range runs from the row touched last, readme.md.
        inject_modifiers(ModifiersState::SHIFT);
        tap_row(browser, "main.rs")?;
        assert_eq!(picked(browser), ["main.rs", "movie.mkv", "readme.md"]);
        check_colors(CHECK_3)?;

        inject_modifiers(ModifiersState::empty());
        let below =
            from_main(move || browser.row_center("readme.md")).map(|row| Point::new(row.x, row.y + 120.0));
        tap_at(below.unwrap_or_default());
        assert_eq!(picked(browser), Vec::<String>::new());
        assert_eq!(view.status.text(), "picked: ");
        assert_eq!(from_main(move || view.changes), 7);
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             376   56 - #c5c8ce
             192   84 - #2f8bf5
             236   84 - #404041
             504  116 - #ffffff
             232  120 - #3a3a3c
              16  144 - #8f8f96
              88  144 - #9a9a9c
             192  148 - #2f8bf5
             188  200 - #64a8f7
             268  264 - #1c1c1e
             504  264 - #e4e4e6
             524  264 - #d3d3d6
             184  288 - #6e6e76
             416  324 - #c2c2c5
             264  328 - #ffffff
             236  352 - #9ec5ff
             504  352 - #4b93ff
             420  356 - #0a6cff
             184  360 - #0a6cff
             584  368 - #0a6cff
               4  396 - #f4f4f6
             592  556 - #ffffff
             376  568 - #597c95
              72  576 - #0c1114
              32  580 - #597c95
              84  580 - #000000
             112  580 - #000000
             160  580 - #597c95
             484  592 - #597c95
";

const CHECK_2: &str = r"
             328    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             580   20 - #6e6e76
             376   56 - #c5c8ce
              20   80 - #6e6e76
             236   84 - #404041
             192   88 - #2f8bf5
             232  120 - #3a3a3c
             188  140 - #64a8f7
              16  144 - #8f8f96
              76  144 - #1c1c1e
             500  148 - #ffffff
             376  168 - #0a6cff
             184  240 - #2f8bf5
             540  260 - #0a6cff
             264  264 - #86b6ff
             212  296 - #e4e4e5
             444  320 - #85858c
             264  328 - #ffffff
             236  352 - #9ec5ff
             536  356 - #0a6cff
               4  364 - #f4f4f6
             356  368 - #0a6cff
             592  556 - #ffffff
              32  580 - #597c95
              72  580 - #000000
             116  580 - #415a6d
             152  580 - #597c95
             276  580 - #000000
             192  584 - #1c272f
             436  592 - #597c95
";

const CHECK_3: &str = r"
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             424   24 - #c3c3c9
             376   56 - #c5c8ce
             516   56 - #6e6e76
             192   84 - #2f8bf5
             236   84 - #404041
              16   88 - #dcdce3
             232  116 - #1c1c1e
             504  116 - #ffffff
             188  140 - #64a8f7
              88  144 - #9a9a9c
             192  204 - #2f8bf5
             496  260 - #85858c
             524  264 - #d3d3d6
             336  280 - #0a6cff
             232  296 - #aecfff
             416  324 - #72aaff
               4  352 - #f4f4f6
             236  352 - #9ec5ff
             276  352 - #ffffff
             536  356 - #0a6cff
             184  360 - #0a6cff
             452  500 - #ffffff
              72  576 - #0c1114
              32  580 - #597c95
             164  580 - #010101
             236  580 - #2d3e4b
             256  580 - #000000
             304  580 - #597c95
             592  592 - #597c95
";

const CHECK_4: &str = r"
             300    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             228   56 - #1c1c1e
             376   56 - #c5c8ce
              52   84 - #dcdce3
             192   84 - #2f8bf5
             236   84 - #404041
             504  116 - #ffffff
             232  120 - #3a3a3c
             188  140 - #64a8f7
              16  144 - #8f8f96
              88  144 - #9a9a9c
             192  148 - #2f8bf5
             188  200 - #64a8f7
             252  236 - #ffffff
             440  264 - #818188
             524  264 - #d3d3d6
             184  288 - #6e6e76
             268  356 - #adadae
             504  356 - #d8d8da
             556  356 - #d1d1d3
             184  360 - #ffffff
               4  396 - #f4f4f6
             336  556 - #ffffff
              72  576 - #0c1114
              32  580 - #597c95
              56  580 - #597c95
              72  580 - #000000
             208  592 - #597c95
             592  592 - #597c95
";
