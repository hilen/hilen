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
             392    8 - #3b3b40
             580   20 - #9a9aa2
             124   24 - #aaaaac
             228   56 - #f2f2f4
             188   80 - #4c84c7
             504   80 - #1e1e20
             192   84 - #5aa5ff
             236   84 - #d1d1d3
             232  120 - #d6d6d8
             188  140 - #4c84c7
              16  144 - #7e7e84
              88  144 - #7b7b7e
             192  172 - #5aa5ff
             380  192 - #2f81f7
             188  200 - #cbdffd
             512  200 - #2f81f7
             192  204 - #ffffff
             588  216 - #2f81f7
             232  236 - #434345
             252  236 - #1e1e20
             184  240 - #5aa5ff
             276  352 - #5c5c64
             180  356 - #5c5c64
             420  356 - #1e1e20
             572  356 - #36363a
               4  396 - #27272a
             244  472 - #1e1e20
             336  552 - #3b3b40
              32  576 - #49494d
             456  576 - #323236
             528  576 - #2f81f7
             564  576 - #b8d4fc
";

const CHECK_2: &str = r"
             580   16 - #434348
             116   20 - #58585a
             452   24 - #1e1e20
             372   60 - #4b84c8
             220   64 - #3d628f
             536   72 - #32547c
              20   80 - #9a9aa2
             256   80 - #4c84c8
              28  116 - #9a9aa2
             508  120 - #ccccce
              88  144 - #7b7b7e
             368  156 - #4b84c8
             244  160 - #5aa5ff
             388  160 - #5aa5ff
             504  160 - #ffffff
             536  164 - #819dc4
             536  172 - #819dc4
             388  212 - #f2f2f4
             472  228 - #2f81f7
             524  252 - #5c5c64
             252  256 - #5aa5ff
             236  308 - #e6e6e8
             544  308 - #5c5c64
               4  348 - #27272a
             368  352 - #2e2e31
             236  356 - #1e1e20
             508  376 - #5c5c64
             248  552 - #3b3b40
              40  576 - #74747a
             444  576 - #808083
             528  576 - #2f81f7
             564  576 - #b8d4fc
";

const CHECK_3: &str = r"
             580   16 - #d4d4d7
             116   20 - #bcbcbd
             424   24 - #c3c3c9
             368   60 - #63a8f7
             244   64 - #2f8bf5
             536   68 - #6a93c2
             536   76 - #6a93c2
              20   80 - #6e6e76
              68   84 - #818186
              28  116 - #6e6e76
             508  116 - #525254
              16  144 - #8f8f96
              76  144 - #1c1c1e
             372  156 - #63a8f7
             244  160 - #2f8bf5
             536  172 - #7596c6
             228  184 - #2f8bf5
             388  212 - #1c1c1e
             384  216 - #4e4e4f
             472  228 - #0a6cff
             256  260 - #63a8f8
             532  260 - #b4b4bc
             220  272 - #97c5fb
             236  356 - #ffffff
             384  372 - #ffffff
             536  372 - #bebebf
             200  552 - #d9d9de
              84  576 - #a6a6ab
             444  576 - #98989b
             528  576 - #0a6cff
             564  576 - #abcdff
               4  592 - #f4f4f6
";
