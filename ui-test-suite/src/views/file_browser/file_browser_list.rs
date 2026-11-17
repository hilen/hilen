use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, Setup, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use super::{names, sample};

/// How the browser draws a folder: the toolbar, the sidebar of places,
/// the columns, folders before files. With the sidebar gone the list is
/// wide enough for the date column too.
#[view]
struct FileBrowserList {
    #[init]
    browser: FileBrowser,
}

impl Setup for FileBrowserList {
    fn setup(self: Weak<Self>) {
        self.browser.place().back();
        self.browser.set_open_on_tap(false).set_source(sample());
    }
}

impl ViewTest for FileBrowserList {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;

        assert_eq!(
            names(browser),
            [
                "Documents",
                "Empty",
                "Many",
                "Music",
                "Photos",
                "Projects",
                "archive.zip",
                "main.rs",
                "movie.mkv",
                "readme.md"
            ]
        );
        from_main(move || {
            assert_eq!(browser.crumb_titles(), ["Disk"]);
            assert_eq!(browser.place_titles(), ["Disk", "Photos", "Documents"]);
            assert!(!browser.can_go_up());
        });
        check_colors(CHECK_1)?;

        from_main(move || {
            browser.set_sidebar_hidden(true);
        });
        check_colors(CHECK_2)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
             204    8 - #d9d9de
             316    8 - #d9d9de
             516    8 - #d9d9de
             124   24 - #676768
             424   24 - #c3c3c9
             436   56 - #7c7c83
              68   84 - #818186
             236   84 - #404041
             240   84 - #1c1c1e
              24   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             184  176 - #ffffff
             504  176 - #ffffff
             252  236 - #ffffff
             220  264 - #1c1c1e
             268  264 - #1c1c1e
             444  264 - #6e6e76
             524  264 - #d3d3d6
             544  264 - #d8d8da
             232  296 - #676768
             444  320 - #85858c
             416  324 - #c2c2c5
             532  324 - #99999e
             212  356 - #e8e8e8
             268  356 - #adadae
             496  356 - #d5d5d7
             556  356 - #d1d1d3
               4  380 - #f4f4f6
               4  592 - #f4f4f6
             408  592 - #ffffff
";

const CHECK_2: &str = r"
             200    8 - #d9d9de
             284    8 - #d9d9de
             516    8 - #d9d9de
             116   20 - #bcbcbd
             448   24 - #e8e8eb
             296   56 - #7c7c83
             376   56 - #6e6e76
              72   84 - #404041
             512   84 - #6e6e76
              68  116 - #1c1c1e
              68  120 - #3a3a3c
             364  176 - #ffffff
              20  204 - #ffffff
             516  204 - #b5b5b9
              68  236 - #d8d8d8
             104  264 - #1c1c1e
             300  264 - #818188
             364  264 - #e4e4e6
             384  264 - #d3d3d6
             508  268 - #adadb2
             540  292 - #87878e
              68  296 - #676768
             296  324 - #ffffff
             392  324 - #99999e
              56  356 - #bcbcbd
             112  356 - #1c1c1e
             356  356 - #d5d5d7
             416  356 - #d1d1d3
             492  356 - #ffffff
             368  572 - #ffffff
             148  592 - #ffffff
             592  592 - #ffffff
";
