use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{FileBrowser, Label, Modifiers, Point, Setup, TextAlignment, ViewData, ViewTest, view},
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

        inject_modifiers(Modifiers::META);
        tap_row(browser, "archive.zip")?;
        assert_eq!(picked(browser), ["archive.zip", "readme.md"]);
        tap_row(browser, "Music")?;
        assert_eq!(picked(browser), ["Music", "archive.zip", "readme.md"]);
        check_colors(CHECK_2)?;
        tap_row(browser, "readme.md")?;
        assert_eq!(picked(browser), ["Music", "archive.zip"]);

        // The range runs from the row touched last, readme.md.
        inject_modifiers(Modifiers::SHIFT);
        tap_row(browser, "main.rs")?;
        assert_eq!(picked(browser), ["main.rs", "movie.mkv", "readme.md"]);
        check_colors(CHECK_3)?;

        inject_modifiers(Modifiers::empty());
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
              116    8 - #d9d9de
              320    8 - #d9d9de
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              516   56 - #6e6e76
               52   84 - #dcdce3
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              504  176 - #ffffff
              184  236 - #ffffff
              232  236 - #d8d8d8
              508  260 - #85858c
              268  264 - #1c1c1e
              440  264 - #818188
              232  296 - #676768
              444  324 - #6e6e76
              532  324 - #99999e
              264  328 - #ffffff
              236  352 - #9ec5ff
              508  356 - #0a6cff
              536  356 - #0a6cff
                4  360 - #f4f4f6
              324  368 - #0a6cff
              396  368 - #0a6cff
              400  556 - #ffffff
               32  580 - #597c95
               84  580 - #000000
              156  580 - #000001
              280  592 - #597c95
              592  592 - #597c95
";

const CHECK_2: &str = r"
              116    8 - #d9d9de
              320    8 - #d9d9de
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              516   56 - #6e6e76
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              500  148 - #ffffff
              584  160 - #0d6eff
              188  176 - #0a6cff
              388  188 - #0a6cff
              184  236 - #ffffff
              232  236 - #d8d8d8
              444  260 - #d8e7ff
              540  260 - #0a6cff
              232  296 - #676768
              532  324 - #99999e
              264  328 - #ffffff
              236  352 - #9ec5ff
              420  356 - #0a6cff
                4  360 - #f4f4f6
              592  556 - #ffffff
              356  560 - #597c95
               84  576 - #000001
               32  580 - #597c95
              152  580 - #597c95
              204  580 - #597c95
              276  580 - #000000
              192  584 - #1c272f
              436  592 - #597c95
";

const CHECK_3: &str = r"
              592    4 - #f4f4f6
              328    8 - #d9d9de
              508    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
              436   56 - #7c7c83
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              184  176 - #ffffff
              504  176 - #ffffff
              232  236 - #d8d8d8
              444  260 - #85858c
              268  264 - #1c1c1e
              536  264 - #6e6e76
              320  280 - #0a6cff
              408  280 - #0a6cff
              184  292 - #0a6cff
              232  296 - #aecfff
              444  320 - #d8e7ff
                4  352 - #f4f4f6
              236  352 - #9ec5ff
              276  352 - #ffffff
              420  356 - #0a6cff
              536  356 - #0a6cff
              448  560 - #597c95
               72  576 - #0c1114
              160  580 - #597c95
              228  580 - #000000
              300  580 - #000001
              592  592 - #597c95
";

const CHECK_4: &str = r"
              116    8 - #d9d9de
              312    8 - #d9d9de
              420   24 - #c3c3c9
              436   56 - #7c7c83
              516   56 - #6e6e76
               52   84 - #dcdce3
              236   84 - #404041
               24   88 - #dcdce3
              148   92 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              184  176 - #ffffff
              504  176 - #ffffff
              232  236 - #d8d8d8
              252  236 - #ffffff
              444  260 - #85858c
              496  260 - #85858c
              220  264 - #1c1c1e
              268  264 - #1c1c1e
              424  264 - #ffffff
              536  264 - #6e6e76
              444  324 - #6e6e76
              532  324 - #99999e
                4  356 - #f4f4f6
              248  356 - #e4e4e5
              128  444 - #f4f4f6
              416  556 - #ffffff
              244  568 - #597c95
               32  580 - #597c95
               72  580 - #000000
              500  592 - #597c95
              592  592 - #597c95
";
