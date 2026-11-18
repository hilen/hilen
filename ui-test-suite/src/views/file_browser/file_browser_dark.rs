use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        FileBrowser, FileBrowserControl, FileBrowserMode, Setup, Theme, ThemeMode, UIManager, ViewData,
        ViewTest, view,
    },
    ui_test::check_colors,
};

use super::{sample, tap_control, tap_row};

/// The dark theme. Every color of the browser is a pair, and the tinted
/// icons are drawn again when the theme changes under an open browser.
#[view]
struct FileBrowserDark {
    #[init]
    browser: FileBrowser,
}

impl Setup for FileBrowserDark {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color("#101012");
        self.browser.place().back();
        self.browser
            .set_open_on_tap(false)
            .set_mode(FileBrowserMode::PickFolder)
            .set_source(sample());
    }
}

impl ViewTest for FileBrowserDark {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;

        tap_row(browser, "Photos")?;
        // The browser is on screen in light when the theme turns.
        from_main(|| Theme::set_mode(ThemeMode::Dark));
        check_colors(CHECK_1)?;

        tap_control(browser, FileBrowserControl::GridSwitch);
        tap_control(browser, FileBrowserControl::HiddenSwitch);
        check_colors(CHECK_2)?;

        from_main(|| Theme::set_mode(ThemeMode::Light));
        check_colors(CHECK_3)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
             116    8 - #3b3b40
             124   24 - #aaaaac
             228   56 - #f2f2f4
             436   56 - #8e8e95
             516   56 - #9a9aa2
              68   84 - #939397
             236   84 - #d1d1d3
              24   88 - #3d3d43
             232  116 - #f2f2f4
             240  116 - #9b9b9d
             232  120 - #d6d6d8
              76  144 - #f2f2f4
              88  144 - #7b7b7e
             380  192 - #2f81f7
             512  200 - #2f81f7
             184  204 - #2f81f7
             588  216 - #2f81f7
             232  236 - #434345
             252  236 - #1e1e20
             424  264 - #1e1e20
             500  292 - #585860
               4  356 - #27272a
             268  356 - #343439
             420  356 - #1e1e20
             544  356 - #5c5c64
             244  472 - #1e1e20
             336  552 - #3b3b40
              32  576 - #49494d
              40  576 - #74747a
             456  576 - #323236
             528  576 - #2f81f7
             564  576 - #b8d4fc
";

const CHECK_2: &str = r"
              264    8 - #3b3b40
              124   24 - #aaaaac
              420   24 - #4f4f55
               36   60 - #27272a
              396   76 - #5aa5ff
              504   76 - #5aa5ff
              352  116 - #1e1e20
              508  116 - #bfbfc1
              508  120 - #ccccce
              580  140 - #2f81f7
               76  144 - #f2f2f4
              504  160 - #ffffff
              504  180 - #ffffff
              388  212 - #f2f2f4
              384  216 - #c3c3c5
              584  228 - #2f81f7
              524  252 - #5c5c64
              236  308 - #e6e6e8
              496  312 - #53535a
              592  336 - #1e1e20
              388  348 - #5c5c64
                4  356 - #27272a
              236  356 - #1e1e20
              388  376 - #5c5c64
              260  552 - #3b3b40
               40  576 - #74747a
               84  576 - #6a6a70
              432  576 - #323236
              456  576 - #323236
              528  576 - #2f81f7
              564  576 - #b8d4fc
              536  580 - #2f81f7
";

const CHECK_3: &str = r"
              272    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
               52   84 - #dcdce3
              504   84 - #2f8bf5
               24   88 - #dcdce3
              376  116 - #d3d3d4
              508  116 - #525254
              508  120 - #454547
               76  144 - #1c1c1e
              588  156 - #0a6cff
              228  160 - #ffffff
              396  164 - #2f8bf5
              388  212 - #1c1c1e
              384  216 - #4e4e4f
              492  228 - #0a6cff
              584  228 - #0a6cff
              524  272 - #ffffff
              236  308 - #28282a
              388  348 - #b4b4bc
              520  352 - #ffffff
                4  356 - #f4f4f6
              236  356 - #ffffff
              388  376 - #b4b4bc
              352  404 - #b4b4bc
              408  552 - #d9d9de
               40  576 - #9a9aa0
               84  576 - #a6a6ab
              528  576 - #0a6cff
              564  576 - #abcdff
              536  580 - #0a6cff
              292  592 - #f4f4f6
";
