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
             308    8 - #d9d9de
             104   16 - #d9d9de
             388   20 - #ffffff
             448   20 - #e8e8eb
             188   24 - #8a8a91
             424   24 - #c3c3c9
             228   56 - #1c1c1e
             516   56 - #6e6e76
             184   80 - #ffffff
             444   80 - #85858c
              68   84 - #8e8e90
             212   84 - #e8e8e8
             228   84 - #d2d2d2
              24   88 - #f4f4f6
              84  144 - #cdcdcf
             332  320 - #ffffff
             592  344 - #ffffff
             592  560 - #597c95
              84  576 - #151d22
             156  576 - #07090b
             376  576 - #2c3d4a
              32  580 - #597c95
              56  580 - #000000
             128  580 - #2e414e
             192  580 - #000000
             240  580 - #597c95
             256  580 - #000000
             300  580 - #486478
             344  580 - #2d3e4b
             396  580 - #425c6e
             408  580 - #597c95
             504  592 - #597c95
";

const CHECK_2: &str = r"
             384    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             172   24 - #99999f
             244   24 - #e7e7e8
             320   24 - #a3a3a4
             424   24 - #c3c3c9
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
              68   84 - #8e8e90
             188   84 - #ffffff
              24   88 - #f4f4f6
              52  112 - #f4f4f6
              76  144 - #1c1c1e
             332  312 - #ffffff
               4  356 - #f4f4f6
             592  388 - #ffffff
             420  560 - #597c95
             592  560 - #597c95
             140  576 - #2d3e4b
             156  576 - #07090b
              44  580 - #000001
              76  580 - #597c95
             104  580 - #364b5a
             128  580 - #2e414e
             192  580 - #000000
             240  580 - #597c95
             256  580 - #000000
             300  580 - #486478
             312  580 - #597c95
             528  592 - #597c95
";

const CHECK_3: &str = r"
             300    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             180   24 - #909096
             240   24 - #909096
             448   24 - #e8e8eb
             516   56 - #6e6e76
              24   88 - #f4f4f6
             308  116 - #ec968f
             328  116 - #e05247
             332  116 - #ffffff
             268  120 - #d92d20
             288  120 - #efa6a1
             336  120 - #f3bfbb
             440  120 - #e15b51
             476  120 - #f4c0bc
             500  120 - #d92e21
              84  144 - #cdcdcf
             400  160 - #b4b4b7
             212  348 - #ffffff
             592  412 - #ffffff
             436  560 - #597c95
              84  576 - #151d22
             156  576 - #07090b
              44  580 - #000001
             120  580 - #597c95
             140  580 - #2d3e4b
             248  580 - #384e5e
             292  580 - #000000
             300  580 - #486478
             340  580 - #597c95
             532  592 - #597c95
";
