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
             472    8 - #d9d9de
             104   16 - #d9d9de
             580   20 - #6e6e76
             424   24 - #c3c3c9
             228   56 - #1c1c1e
             376   56 - #c5c8ce
              36   60 - #f4f4f6
             188   80 - #64a8f7
             192   84 - #2f8bf5
             236   84 - #404041
             504  116 - #ffffff
             232  120 - #3a3a3c
             188  140 - #64a8f7
              16  144 - #8f8f96
              88  144 - #9a9a9c
             192  148 - #2f8bf5
             188  200 - #64a8f7
             192  232 - #2f8bf5
             252  236 - #ffffff
             444  260 - #85858c
             180  264 - #6e6e76
             220  264 - #1c1c1e
             524  264 - #d3d3d6
             212  296 - #e4e4e5
             428  324 - #dfdfe0
             180  352 - #6e6e76
             276  356 - #1c1c1e
             504  356 - #d8d8da
             556  356 - #d1d1d3
             572  356 - #c7c7ca
               4  592 - #f4f4f6
             388  592 - #ffffff
";

const CHECK_2: &str = r"
             348    8 - #d9d9de
             420    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
              64   56 - #1c1c1e
             236   56 - #c5c8ce
             476   56 - #73737a
              24   80 - #64a8f7
              72   84 - #404041
              68  120 - #3a3a3c
              24  140 - #64a8f7
              28  172 - #2f8bf5
             540  172 - #87878e
             364  176 - #ffffff
              24  200 - #64a8f7
              28  208 - #2f8bf5
              20  240 - #2f8bf5
             304  260 - #85858c
              56  264 - #1c1c1e
             100  264 - #8c8c8d
             384  264 - #d3d3d6
             404  264 - #d8d8da
             512  264 - #6e6e76
             564  296 - #dcdcde
             296  324 - #ffffff
             364  324 - #d8d8da
             508  328 - #adadb2
             112  356 - #1c1c1e
             416  356 - #d1d1d3
              20  360 - #ffffff
             248  592 - #ffffff
             592  592 - #ffffff
";
