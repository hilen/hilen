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
              116    8 - #d9d9de
              184    8 - #d9d9de
              312    8 - #d9d9de
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              436   56 - #7c7c83
              516   56 - #6e6e76
               52   84 - #dcdce3
              236   84 - #404041
               24   88 - #dcdce3
              148   92 - #dcdce3
              504  116 - #ffffff
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              184  176 - #ffffff
              504  176 - #ffffff
              232  236 - #d8d8d8
              252  236 - #ffffff
              496  260 - #85858c
              508  260 - #85858c
              220  264 - #1c1c1e
              268  264 - #1c1c1e
              444  264 - #6e6e76
              536  264 - #6e6e76
              232  296 - #676768
              444  320 - #85858c
              532  324 - #99999e
              248  356 - #e4e4e5
               12  364 - #f4f4f6
              592  528 - #ffffff
                4  592 - #f4f4f6
              392  592 - #ffffff
";

const CHECK_2: &str = r"
              196    8 - #d9d9de
              272    8 - #d9d9de
              500    8 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
               64   56 - #1c1c1e
              296   56 - #7c7c83
              376   56 - #6e6e76
              476   56 - #73737a
               72   84 - #404041
              508   88 - #adadb2
               68  120 - #3a3a3c
              540  172 - #87878e
               20  176 - #ffffff
              364  176 - #ffffff
              508  208 - #adadb2
               68  236 - #d8d8d8
              484  236 - #ffffff
              356  260 - #85858c
               56  264 - #1c1c1e
              104  264 - #1c1c1e
              300  264 - #818188
              540  268 - #adadb2
              296  324 - #ffffff
              508  328 - #adadb2
              112  352 - #1c1c1e
               84  356 - #e4e4e5
              404  356 - #6e6e76
              592  520 - #ffffff
              200  536 - #ffffff
                4  592 - #ffffff
              396  592 - #ffffff
";
