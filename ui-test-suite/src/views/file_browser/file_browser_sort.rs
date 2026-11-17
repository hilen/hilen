use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Label, Setup, SortKey, TextAlignment, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use super::{names, picked, sample, tap_control, tap_row};

const FOLDERS: [&str; 6] = ["Documents", "Empty", "Many", "Music", "Photos", "Projects"];

fn rows(folders: &[&str], files: &[&str]) -> Vec<String> {
    folders.iter().chain(files).map(ToString::to_string).collect()
}

/// A tap on a column title sorts by it, a second tap turns the order
/// around. Folders stay above the files either way, and a picked entry
/// stays picked while the rows move.
#[view]
struct FileBrowserSort {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl FileBrowserSort {
    fn show_sort(self: Weak<Self>) {
        let sort = self.browser.sort();
        let way = if sort.ascending { "up" } else { "down" };
        self.status.set_text(format!("sorted by {:?}, {way}", sort.key));
    }
}

impl Setup for FileBrowserSort {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        // No sidebar, so the list is wide enough for the date column.
        self.browser
            .set_open_on_tap(false)
            .set_sidebar_hidden(true)
            .set_source(sample());

        self.status.set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
        self.show_sort();
    }
}

impl ViewTest for FileBrowserSort {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let sort_by = move |key: SortKey| {
            tap_control(browser, FileBrowserControl::Header(key));
            from_main(move || view.show_sort());
        };

        tap_row(browser, "main.rs")?;

        sort_by(SortKey::Size);
        assert_eq!(
            names(browser),
            rows(&FOLDERS, &["readme.md", "main.rs", "archive.zip", "movie.mkv"])
        );
        assert_eq!(picked(browser), ["main.rs"]);
        check_colors(CHECK_1)?;

        sort_by(SortKey::Size);
        let mut reversed = FOLDERS;
        reversed.reverse();
        assert_eq!(
            names(browser),
            rows(&reversed, &["movie.mkv", "archive.zip", "main.rs", "readme.md"])
        );
        assert_eq!(picked(browser), ["main.rs"]);
        check_colors(CHECK_2)?;

        sort_by(SortKey::Kind);
        assert_eq!(
            names(browser),
            rows(&FOLDERS, &["readme.md", "movie.mkv", "main.rs", "archive.zip"])
        );

        sort_by(SortKey::Modified);
        assert_eq!(
            names(browser),
            rows(
                &["Many", "Documents", "Music", "Photos", "Empty", "Projects"],
                &["movie.mkv", "readme.md", "archive.zip", "main.rs"]
            )
        );
        check_colors(CHECK_3)?;

        sort_by(SortKey::Name);
        assert_eq!(
            names(browser),
            rows(&FOLDERS, &["archive.zip", "main.rs", "movie.mkv", "readme.md"])
        );
        assert_eq!(picked(browser), ["main.rs"]);

        Ok(())
    }
}

const CHECK_1: &str = r"
             156    8 - #d9d9de
             360    8 - #d9d9de
             580   20 - #6e6e76
             440   24 - #ffffff
             296   56 - #323234
              28   84 - #2f8bf5
             512   84 - #6e6e76
              68  120 - #3a3a3c
              24  140 - #64a8f7
              28  144 - #2f8bf5
             540  172 - #87878e
              24  200 - #64a8f7
             564  260 - #85858c
             112  264 - #1c1c1e
             516  264 - #b5b5b9
             288  292 - #edf4ff
              68  296 - #aecfff
             384  296 - #0a6cff
              20  300 - #0a6cff
             180  308 - #0a6cff
             456  308 - #0a6cff
             384  324 - #d3d3d6
             540  328 - #adadb2
             288  352 - #dfdfe0
              48  356 - #e4e4e5
             592  476 - #ffffff
             456  556 - #ffffff
             296  568 - #597c95
              40  580 - #597c95
              68  580 - #597c95
             136  580 - #0f151a
             572  592 - #597c95
";

const CHECK_2: &str = r"
             220    8 - #d9d9de
             580   20 - #6e6e76
             124   24 - #676768
             424   24 - #c3c3c9
             296   56 - #323234
              28   84 - #2f8bf5
             540   84 - #87878e
              28  144 - #2f8bf5
             540  144 - #87878e
             364  176 - #ffffff
              24  200 - #64a8f7
             508  208 - #adadb2
              76  236 - #1c1c1e
              88  236 - #a1a1a1
             364  264 - #d8d8da
             284  296 - #ffffff
             384  296 - #d3d3d6
              68  324 - #aecfff
             484  324 - #0a6cff
             516  324 - #87b7ff
             556  324 - #0a6cff
             508  328 - #94bfff
             180  336 - #0a6cff
              20  356 - #dbdbdd
             112  356 - #1c1c1e
             416  356 - #d1d1d3
             592  556 - #ffffff
              40  580 - #597c95
              68  580 - #597c95
             172  580 - #000000
             116  584 - #1c272f
             384  592 - #597c95
";

const CHECK_3: &str = r"
             348    4 - #f4f4f6
             232    8 - #d9d9de
             116   20 - #bcbcbd
             580   20 - #6e6e76
             424   24 - #c3c3c9
              24   80 - #64a8f7
              88  116 - #a1a1a1
             364  116 - #ffffff
              24  140 - #64a8f7
              60  144 - #434345
              72  176 - #373738
             540  176 - #87878e
              68  204 - #1c1c1e
              28  208 - #2f8bf5
             392  264 - #99999e
             512  264 - #6e6e76
              20  296 - #dbdbdd
              84  296 - #e4e4e5
             280  296 - #ffffff
             416  296 - #d1d1d3
             356  320 - #85858c
             484  352 - #0a6cff
              68  356 - #aecfff
             384  356 - #0a6cff
             564  356 - #4690ff
             188  368 - #0a6cff
              72  576 - #141b21
              40  580 - #597c95
             100  580 - #000000
             136  580 - #000000
             180  580 - #597c95
             472  592 - #597c95
";
