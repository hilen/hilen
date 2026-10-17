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
             136    8 - #d9d9de
             316    8 - #d9d9de
             448   20 - #e8e8eb
             424   24 - #c3c3c9
             516   56 - #6e6e76
             236   84 - #404041
              24   88 - #dcdce3
             240  116 - #79797b
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             184  236 - #ffffff
             444  260 - #85858c
             268  264 - #1c1c1e
             524  264 - #d3d3d6
             416  324 - #c2c2c5
             532  324 - #99999e
             264  328 - #ffffff
             236  352 - #9ec5ff
             268  356 - #62a1ff
             420  356 - #0a6cff
             508  356 - #0a6cff
             572  356 - #68a4ff
               4  360 - #f4f4f6
             172  364 - #0a6cff
             340  368 - #0a6cff
             368  556 - #ffffff
              32  580 - #597c95
              84  580 - #000000
             132  580 - #2d3e4b
             156  580 - #000001
             476  592 - #597c95
             592  592 - #597c95
";

const CHECK_2: &str = r"
             128    8 - #d9d9de
             316    8 - #d9d9de
             424   24 - #c3c3c9
             516   56 - #6e6e76
             236   84 - #404041
              24   88 - #dcdce3
             240  116 - #79797b
              88  144 - #9a9a9c
             504  176 - #0a6cff
             300  184 - #0a6cff
             388  188 - #0a6cff
             184  236 - #ffffff
             444  260 - #d8e7ff
             540  260 - #0a6cff
             264  264 - #86b6ff
             416  324 - #c2c2c5
             532  324 - #99999e
             264  328 - #ffffff
               4  356 - #f4f4f6
             220  356 - #5297ff
             496  356 - #5197ff
             572  356 - #68a4ff
             420  488 - #ffffff
              72  576 - #0c1114
             296  576 - #2e404d
              32  580 - #597c95
             112  580 - #597c95
             240  580 - #000001
             276  580 - #000000
             192  584 - #1c272f
             400  592 - #597c95
             532  592 - #597c95
";

const CHECK_3: &str = r"
             316    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             424   24 - #c3c3c9
             228   56 - #1c1c1e
             516   56 - #6e6e76
              68   84 - #818186
             236   84 - #404041
             240  116 - #79797b
             504  116 - #ffffff
              88  144 - #9a9a9c
             232  236 - #d8d8d8
             444  260 - #85858c
             524  264 - #d3d3d6
             336  280 - #0a6cff
             184  292 - #0a6cff
             232  296 - #aecfff
             416  324 - #72aaff
             504  324 - #4b93ff
             532  324 - #b7d4ff
             276  352 - #ffffff
               4  356 - #f4f4f6
             220  356 - #5297ff
             504  356 - #4b93ff
             536  356 - #0a6cff
             352  368 - #0a6cff
             448  512 - #ffffff
              72  576 - #0c1114
             176  580 - #2f424f
             228  580 - #000000
             300  580 - #000001
             592  592 - #597c95
";

const CHECK_4: &str = r"
             316    8 - #d9d9de
             124   24 - #676768
             424   24 - #c3c3c9
             516   56 - #6e6e76
               8   76 - #dcdce3
              68   84 - #818186
             236   84 - #404041
             240   84 - #1c1c1e
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             504  176 - #ffffff
             252  236 - #ffffff
             444  260 - #85858c
             220  264 - #1c1c1e
             268  264 - #1c1c1e
             524  264 - #d3d3d6
             544  264 - #d8d8da
             232  296 - #676768
             416  324 - #c2c2c5
             532  324 - #99999e
               4  336 - #f4f4f6
             212  356 - #e8e8e8
             268  356 - #adadae
             420  356 - #ffffff
             496  356 - #d5d5d7
             556  356 - #d1d1d3
             484  496 - #ffffff
             356  556 - #ffffff
              32  580 - #597c95
              72  580 - #000000
             216  592 - #597c95
             592  592 - #597c95
";
