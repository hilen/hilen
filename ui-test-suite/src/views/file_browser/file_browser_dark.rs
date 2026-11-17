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
             272    8 - #3b3b40
             124   24 - #aaaaac
             424   24 - #4f4f55
              68   84 - #939397
             396   84 - #5aa5ff
              24   88 - #3d3d43
             508  112 - #79797b
             508  120 - #ccccce
             580  140 - #2f81f7
              84  144 - #4c4c4e
             504  160 - #ffffff
             396  164 - #5aa5ff
             504  180 - #ffffff
             388  212 - #f2f2f4
             384  216 - #c3c3c5
             588  216 - #2f81f7
             524  272 - #1e1e20
             236  308 - #e6e6e8
             592  340 - #1e1e20
             388  348 - #5c5c64
             516  352 - #1e1e20
               4  356 - #27272a
             236  356 - #1e1e20
             388  376 - #5c5c64
             236  404 - #36363a
             352  404 - #5c5c64
             248  552 - #3b3b40
              40  576 - #74747a
             444  576 - #808083
             528  576 - #2f81f7
             564  576 - #b8d4fc
             536  580 - #2f81f7
";

const CHECK_3: &str = r"
             272    8 - #d9d9de
             388   20 - #ffffff
             448   20 - #e8e8eb
             124   24 - #676768
             424   24 - #c3c3c9
             504   64 - #2f8bf5
             396   68 - #2f8bf5
              68   84 - #818186
              24   88 - #dcdce3
             376  116 - #d3d3d4
             508  120 - #454547
              76  144 - #1c1c1e
             468  192 - #0a6cff
             388  212 - #1c1c1e
             384  216 - #4e4e4f
             584  228 - #0a6cff
             524  256 - #b4b4bc
             544  308 - #b4b4bc
             388  348 - #b4b4bc
             520  352 - #ffffff
             236  356 - #ffffff
               4  360 - #f4f4f6
             388  376 - #b4b4bc
             236  404 - #e2e2e5
             200  552 - #d9d9de
              40  576 - #9a9aa0
              84  576 - #a6a6ab
             444  576 - #98989b
             528  576 - #0a6cff
             564  576 - #abcdff
             536  580 - #0a6cff
               4  592 - #f4f4f6
";
