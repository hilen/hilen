use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_named_key},
    window::NamedKey,
};

use super::{double_tap_row, names, picked, sample, tap_control, tap_row};

/// The grid of icons, the second way to show a folder. The switch in
/// the toolbar turns it on and off, and in it the arrows move sideways
/// too.
#[view]
struct FileBrowserGrid {
    #[init]
    browser: FileBrowser,
}

impl Setup for FileBrowserGrid {
    fn setup(self: Weak<Self>) {
        self.browser.place().back();
        self.browser.set_open_on_tap(false).set_source(sample());
    }
}

impl ViewTest for FileBrowserGrid {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;

        tap_control(browser, FileBrowserControl::GridSwitch);
        assert!(from_main(move || browser.is_grid()));
        assert_eq!(names(browser).len(), 10);
        check_colors(CHECK_1)?;

        tap_row(browser, "Documents")?;
        inject_named_key(NamedKey::ArrowRight);
        assert_eq!(picked(browser), ["Empty"]);
        // Down is one row of the grid, 3 cells here.
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), ["Photos"]);
        inject_named_key(NamedKey::ArrowLeft);
        assert_eq!(picked(browser), ["Music"]);
        check_colors(CHECK_2)?;

        double_tap_row(browser, "Photos")?;
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);
        assert!(from_main(move || browser.is_grid()));
        check_colors(CHECK_3)?;

        tap_control(browser, FileBrowserControl::GridSwitch);
        assert!(!from_main(move || browser.is_grid()));
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
             136    8 - #d9d9de
             316    8 - #d9d9de
             580   16 - #d4d4d7
             452   24 - #f9f9f9
             220   64 - #97c5fb
             388   64 - #2f8bf5
             528   64 - #2f8bf5
             536   72 - #6a93c2
             256   76 - #63a8f8
              20   80 - #6e6e76
             536   80 - #6a93c2
             364  116 - #484849
             516  116 - #d6d6d6
              16  144 - #8f8f96
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             372  156 - #63a8f7
             220  160 - #97c5fb
             256  164 - #63a8f8
             536  168 - #6a93c2
             228  184 - #2f8bf5
             392  184 - #2f8bf5
             228  248 - #6e6e76
             528  256 - #939399
             252  276 - #6e6e76
             372  276 - #ffffff
             504  276 - #a4a4a6
             528  308 - #bfbfbf
             236  344 - #6e6e76
             232  404 - #a8a8a9
               4  592 - #f4f4f6
             592  592 - #ffffff
";

const CHECK_2: &str = r"
             136    8 - #d9d9de
             580   16 - #d4d4d7
             452   24 - #f9f9f9
             388   64 - #2f8bf5
             256   68 - #63a8f8
             536   68 - #6a93c2
             256   76 - #63a8f8
             504   76 - #2f8bf5
             536   76 - #6a93c2
              20   80 - #6e6e76
             364  116 - #484849
             516  116 - #d6d6d6
              16  144 - #8f8f96
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             228  156 - #c2dbff
             372  156 - #63a8f7
             220  160 - #84b5ff
             388  160 - #2f8bf5
             536  168 - #6a93c2
             256  172 - #c2daff
             172  224 - #0a6cff
             304  224 - #0a6cff
             528  256 - #939399
             252  276 - #6e6e76
             372  276 - #ffffff
             504  276 - #a4a4a6
             528  308 - #bfbfbf
             236  344 - #6e6e76
             232  404 - #a8a8a9
               4  592 - #f4f4f6
             592  592 - #ffffff
";

const CHECK_3: &str = r"
             444    8 - #d9d9de
             580   16 - #d4d4d7
             116   20 - #d4d4d7
             172   20 - #a3a3a4
             392   20 - #ffffff
             228   60 - #63a8f7
             388   60 - #929298
             220   64 - #97c5fb
             244   64 - #2f8bf5
             364   64 - #6e6e76
             256   68 - #63a8f8
             516   72 - #6e6e76
             220   76 - #97c5fb
             256   76 - #63a8f8
              20   80 - #6e6e76
             256   80 - #63a8f8
              52   84 - #f4f4f6
             384   84 - #ffffff
             504   84 - #a4a4a6
             536   84 - #6e6e76
             228   88 - #2f8bf5
             244   88 - #2f8bf5
             148  104 - #dcdce3
              16  116 - #dcdce3
             240  116 - #373738
             380  116 - #c0c0c0
             380  120 - #c0c0c0
             528  120 - #aaaaaa
              28  144 - #8f8f96
              76  144 - #1c1c1e
               4  592 - #f4f4f6
             592  592 - #ffffff
";

const CHECK_4: &str = r"
             268    8 - #d9d9de
             332    8 - #d9d9de
             500    8 - #d9d9de
             172   20 - #a3a3a4
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #9e9ea3
             388   24 - #ffffff
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             436   56 - #7c7c83
             516   56 - #6e6e76
             188   80 - #64a8f7
              52   84 - #f4f4f6
             192   84 - #2f8bf5
             224   84 - #bababa
             192   88 - #2f8bf5
              28  112 - #6e6e76
             180  116 - #929298
             416  116 - #ffffff
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
             504  140 - #85858c
              16  144 - #8f8f96
              76  144 - #1c1c1e
              84  144 - #cdcdcf
             232  144 - #1c1c1e
             188  148 - #ffffff
             356  420 - #ffffff
             120  592 - #f4f4f6
             592  592 - #ffffff
";
