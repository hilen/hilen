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
             264    8 - #d9d9de
             356    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             124   24 - #676768
             424   24 - #c3c3c9
              36   60 - #f4f4f6
             396   68 - #2f8bf5
              68   84 - #818186
              24   88 - #dcdce3
             144   96 - #dcdce3
              20  116 - #f4f4f6
             212  116 - #ffffff
             364  116 - #484849
             516  116 - #d6d6d6
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             504  160 - #2f8bf5
             396  180 - #2f8bf5
             500  212 - #5f5f60
             252  260 - #6e6e76
             512  260 - #6e6e76
             372  276 - #ffffff
             360  308 - #d5d5d6
             516  308 - #646465
               4  364 - #f4f4f6
             252  372 - #6e6e76
             232  404 - #a8a8a9
             268  404 - #b9b9b9
               4  592 - #f4f4f6
             376  592 - #ffffff
             592  592 - #ffffff
";

const CHECK_2: &str = r"
             264    8 - #d9d9de
             356    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             124   24 - #676768
              36   60 - #f4f4f6
             396   68 - #2f8bf5
              68   84 - #818186
             144   96 - #dcdce3
              20  116 - #f4f4f6
             212  116 - #ffffff
             364  116 - #484849
             516  116 - #d6d6d6
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             504  160 - #2f8bf5
             172  176 - #0a6cff
             396  180 - #2f8bf5
             304  184 - #0a6cff
             500  212 - #5f5f60
             224  224 - #0a6cff
             512  260 - #6e6e76
             252  276 - #6e6e76
             372  276 - #ffffff
             360  308 - #d5d5d6
             516  308 - #646465
               4  364 - #f4f4f6
             252  372 - #6e6e76
             232  404 - #a8a8a9
             268  404 - #b9b9b9
               4  592 - #f4f4f6
             592  592 - #ffffff
";

const CHECK_3: &str = r"
             232    8 - #d9d9de
             292    8 - #d9d9de
             340    8 - #d9d9de
             516    8 - #d9d9de
             116   20 - #d4d4d7
             172   20 - #a3a3a4
             388   20 - #ffffff
             164   24 - #9f9fa0
             424   24 - #c3c3c9
             440   24 - #ffffff
             528   64 - #ffffff
              68   84 - #8e8e90
             384   84 - #ffffff
              24   88 - #f4f4f6
             512   88 - #6e6e76
             148  104 - #dcdce3
              52  112 - #dcdce3
              20  116 - #dcdce3
              60  116 - #dcdce3
             232  116 - #848485
             240  116 - #373738
             380  116 - #c0c0c0
             380  120 - #c0c0c0
             528  120 - #aaaaaa
              52  140 - #f4f4f6
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             528  360 - #ffffff
              80  376 - #f4f4f6
             300  440 - #ffffff
               4  592 - #f4f4f6
             592  592 - #ffffff
";

const CHECK_4: &str = r"
             244    8 - #d9d9de
             320    8 - #d9d9de
             500    8 - #d9d9de
             172   20 - #a3a3a4
             388   20 - #ffffff
             448   20 - #e8e8eb
             124   24 - #9e9ea3
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
              68   84 - #8e8e90
             184   84 - #ffffff
             224   84 - #bababa
              24   88 - #f4f4f6
             136  100 - #dcdce3
              20  116 - #dcdce3
              52  116 - #dcdce3
             228  116 - #a7a7a8
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
              76  144 - #1c1c1e
             232  144 - #1c1c1e
             412  144 - #dcdcde
             512  144 - #ffffff
             548  144 - #6e6e76
             188  148 - #ffffff
             232  148 - #1c1c1e
             592  388 - #ffffff
             224  408 - #ffffff
               4  548 - #f4f4f6
             412  592 - #ffffff
";
