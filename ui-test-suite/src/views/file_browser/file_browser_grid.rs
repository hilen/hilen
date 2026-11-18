use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, NamedKey, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_named_key},
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
              196    8 - #d9d9de
              272    8 - #d9d9de
              332    8 - #d9d9de
              516    8 - #d9d9de
              388   20 - #ffffff
              124   24 - #676768
              420   24 - #c3c3c9
              504   64 - #2f8bf5
               52   84 - #dcdce3
              396   84 - #2f8bf5
              504   84 - #2f8bf5
               24   88 - #dcdce3
              140   96 - #dcdce3
              212  116 - #ffffff
               76  144 - #1c1c1e
              396  164 - #2f8bf5
              504  172 - #2f8bf5
              500  212 - #5f5f60
              524  256 - #ffffff
              252  260 - #6e6e76
              252  276 - #6e6e76
              372  276 - #ffffff
              512  280 - #6e6e76
              516  308 - #646465
              236  348 - #ffffff
              252  356 - #6e6e76
               12  364 - #f4f4f6
              252  372 - #6e6e76
              176  536 - #ffffff
                4  592 - #f4f4f6
              348  592 - #ffffff
              592  592 - #ffffff
";

const CHECK_2: &str = r"
              204    8 - #d9d9de
              284    8 - #d9d9de
              516    8 - #d9d9de
              388   20 - #ffffff
              124   24 - #676768
              420   24 - #c3c3c9
               52   84 - #dcdce3
              396   84 - #2f8bf5
              504   84 - #2f8bf5
               24   88 - #dcdce3
              140   96 - #dcdce3
              212  116 - #ffffff
              260  140 - #0a6cff
               76  144 - #1c1c1e
              396  164 - #2f8bf5
              504  172 - #2f8bf5
              236  204 - #0a6cff
              304  212 - #0a6cff
              500  212 - #5f5f60
              176  228 - #0a6cff
              512  252 - #6e6e76
              252  260 - #6e6e76
              252  276 - #6e6e76
              372  276 - #ffffff
              524  276 - #ffffff
              516  308 - #646465
              236  348 - #ffffff
                4  368 - #f4f4f6
              252  372 - #6e6e76
                4  592 - #f4f4f6
              348  592 - #ffffff
              592  592 - #ffffff
";

const CHECK_3: &str = r"
              228    8 - #d9d9de
              292    8 - #d9d9de
              340    8 - #d9d9de
              500    8 - #d9d9de
              388   20 - #ffffff
              124   24 - #9e9ea3
              168   24 - #ffffff
              420   24 - #c3c3c9
              528   64 - #ffffff
               52   84 - #f4f4f6
              384   84 - #ffffff
               24   88 - #f4f4f6
              512   88 - #6e6e76
              156  104 - #dcdce3
              108  108 - #dcdce3
               20  112 - #dcdce3
               24  112 - #dcdce3
               52  112 - #dcdce3
               20  116 - #dcdce3
               24  116 - #dcdce3
               52  116 - #dcdce3
               60  116 - #dcdce3
              240  116 - #373738
               52  140 - #f4f4f6
               76  144 - #1c1c1e
              404  276 - #ffffff
              592  332 - #ffffff
               80  376 - #f4f4f6
              300  440 - #ffffff
                4  592 - #f4f4f6
              192  592 - #ffffff
              592  592 - #ffffff
";

const CHECK_4: &str = r"
              188    8 - #d9d9de
              260    8 - #d9d9de
              320    8 - #d9d9de
              476    8 - #d9d9de
              124   24 - #9e9ea3
              420   24 - #c3c3c9
              512   52 - #ffffff
              228   56 - #1c1c1e
              436   56 - #7c7c83
              184   84 - #ffffff
              228   84 - #1c1c1e
              132  100 - #dcdce3
               20  112 - #dcdce3
               52  112 - #dcdce3
               24  116 - #dcdce3
               52  116 - #dcdce3
               60  116 - #dcdce3
              216  116 - #ffffff
              416  116 - #ffffff
              532  116 - #717179
              504  140 - #85858c
               76  144 - #1c1c1e
              232  144 - #1c1c1e
              440  144 - #818188
              548  144 - #6e6e76
              188  148 - #ffffff
              232  148 - #1c1c1e
              228  392 - #ffffff
              592  420 - #ffffff
                4  496 - #f4f4f6
              176  592 - #ffffff
              372  592 - #ffffff
";
