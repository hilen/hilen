use anyhow::{Result, anyhow};
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Label, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint,
        system_input::{press_return, tap, type_text, wait_until},
    },
};

use super::{names, sample, tap_at, tap_control};

/// The path bar. A path too long for it keeps its end and folds the
/// start into one crumb of dots. A tap on the free part turns the bar
/// into a text field, and Return opens the typed path.
#[view]
struct FileBrowserPath {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserPath {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser.set_open_on_tap(false).set_source(sample());
        self.browser.path_changed.val(move |path| {
            self.status.set_text(format!("open folder: {}", path.slash_text()));
        });

        self.status.set_text("open folder: Disk").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserPath {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let crumbs = move || from_main(move || browser.crumb_titles());
        let type_path = move |text: &'static str| -> Result<()> {
            let bar = from_main(move || browser.control_center(FileBrowserControl::PathBar));
            tap(bar.x, bar.y)?;
            wait_until("the path bar is a text field", move || browser.is_editing_path())?;
            type_text(text)?;
            press_return()?;
            wait_until("the path bar shows crumbs again", move || {
                !browser.is_editing_path()
            })
        };

        from_main(move || {
            let deep = ["Disk", "Projects", "engine", "source", "views", "buttons"];
            browser.open(FilePath::new(deep));
        });
        assert_eq!(names(browser), ["round.rs"]);
        // 6 parts do not fit, the end stays and the dots lead to the
        // deepest folded folder.
        assert_eq!(crumbs(), ["...", "source", "views", "buttons"]);
        check_colors(CHECK_1)?;

        let dots = from_main(move || browser.crumb_center("...")).ok_or_else(|| anyhow!("no dots crumb"))?;
        tap_at(dots);
        assert_eq!(crumbs(), ["Disk", "Projects", "engine"]);
        assert_eq!(view.status.text(), "open folder: Disk/Projects/engine");
        checkpoint("the dots opened engine, 3 crumbs fit")?;

        // The field starts with the open path, the typed text goes after.
        type_path("/source")?;
        assert_eq!(view.status.text(), "open folder: Disk/Projects/engine/source");
        assert_eq!(names(browser), ["views"]);
        check_colors(CHECK_2)?;

        // A path that is not there shows the error of the source, and
        // Back leaves it.
        type_path("/nope")?;
        assert_eq!(
            from_main(move || browser.status_text().to_string()),
            "No such folder: Disk/Projects/engine/source/nope"
        );
        check_colors(CHECK_3)?;
        tap_control(browser, FileBrowserControl::Back);
        assert_eq!(names(browser), ["views"]);

        Ok(())
    }
}

const CHECK_1: &str = r"
             272    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             176   24 - #c4c4c8
             512   52 - #ffffff
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             184   80 - #ffffff
             444   80 - #85858c
              20   84 - #f4f4f6
              68   84 - #8e8e90
             212   84 - #e8e8e8
              28  116 - #6e6e76
              16  144 - #8f8f96
              76  144 - #1c1c1e
             332  316 - #ffffff
             592  348 - #ffffff
             592  560 - #597c95
             156  576 - #07090b
             376  576 - #2c3d4a
              32  580 - #597c95
              56  580 - #000000
              76  580 - #597c95
             120  580 - #597c95
             140  580 - #2d3e4b
             256  580 - #000000
             300  580 - #486478
             344  580 - #2d3e4b
             396  580 - #425c6e
             180  584 - #000000
             496  592 - #597c95
";

const CHECK_2: &str = r"
             512    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             164   24 - #96969c
             248   24 - #c6c6c9
             332   24 - #49494a
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             436   56 - #7c7c83
             188   80 - #64a8f7
             512   80 - #ffffff
              52   84 - #f4f4f6
             192   84 - #2f8bf5
             220   84 - #949495
             188   88 - #ffffff
             192   88 - #2f8bf5
              16  144 - #8f8f96
              76  144 - #1c1c1e
             328  316 - #ffffff
             592  360 - #ffffff
             420  560 - #597c95
             140  576 - #2d3e4b
              44  580 - #000001
              80  580 - #000000
             104  580 - #364b5a
             120  580 - #597c95
             192  580 - #000000
             240  580 - #597c95
             300  580 - #486478
             312  580 - #597c95
             540  592 - #597c95
";

const CHECK_3: &str = r"
             340    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             176   24 - #c4c4c7
             448   24 - #e8e8eb
             228   56 - #1c1c1e
             376   56 - #c5c8ce
              52   84 - #f4f4f6
             308  116 - #ec968f
             328  116 - #e05247
             332  116 - #ffffff
             268  120 - #d92d20
             288  120 - #efa6a1
             336  120 - #f3bfbb
             440  120 - #e15b51
             476  120 - #f4c0bc
             500  120 - #d92d20
              16  144 - #8f8f96
              88  144 - #9a9a9c
             208  348 - #ffffff
             592  436 - #ffffff
             592  560 - #597c95
              84  576 - #151d22
             140  576 - #2d3e4b
              44  580 - #000001
             120  580 - #597c95
             248  580 - #384e5e
             300  580 - #486478
             340  580 - #597c95
             180  584 - #000000
             488  592 - #597c95
";
