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
        // Below the status bar of a phone, which takes a tap on the toolbar.
        self.browser.place().t(40).lr(0).b(40);
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
                  4    4 - #597c95
                592    4 - #597c95
                292   48 - #d9d9de
                356   48 - #d9d9de
                104   56 - #d9d9de
                188   64 - #8a8a91
                420   64 - #c3c3c9
                216   92 - #ffffff
                436   96 - #7c7c83
                516   96 - #6e6e76
                184  120 - #ffffff
                444  120 - #85858c
                 52  124 - #f4f4f6
                232  124 - #1c1c1e
                444  124 - #6e6e76
                 24  156 - #f4f4f6
                 76  184 - #1c1c1e
                312  332 - #ffffff
                592  368 - #ffffff
                  4  376 - #f4f4f6
                168  412 - #ffffff
                592  560 - #597c95
                 84  576 - #151d22
                156  576 - #07090b
                 44  580 - #000001
                120  580 - #597c95
                192  580 - #000000
                256  580 - #000000
                292  580 - #000000
                312  580 - #597c95
                380  580 - #597c95
                408  580 - #597c95
";

const CHECK_2: &str = r"
                  4    4 - #597c95
                592    4 - #597c95
                240   48 - #d9d9de
                288   48 - #d9d9de
                480   48 - #d9d9de
                164   60 - #96969c
                388   60 - #ffffff
                124   64 - #9e9ea3
                172   64 - #99999f
                184   64 - #72727a
                332   64 - #49494a
                420   64 - #c3c3c9
                228   96 - #1c1c1e
                436   96 - #7c7c83
                516   96 - #6e6e76
                188  124 - #ffffff
                 24  128 - #f4f4f6
                 52  152 - #f4f4f6
                 76  184 - #1c1c1e
                336  328 - #ffffff
                  4  376 - #f4f4f6
                592  412 - #ffffff
                156  576 - #07090b
                 32  580 - #597c95
                 44  580 - #000001
                 80  580 - #000000
                120  580 - #597c95
                240  580 - #597c95
                292  580 - #000000
                312  580 - #597c95
                180  584 - #000000
                512  592 - #597c95
";

const CHECK_3: &str = r"
                  4    4 - #597c95
                592    4 - #597c95
                296   48 - #d9d9de
                104   56 - #d9d9de
                180   64 - #909096
                184   64 - #6f6f77
                240   64 - #909096
                420   64 - #c3c3c9
                228   96 - #1c1c1e
                436   96 - #7c7c83
                516   96 - #6e6e76
                 24  128 - #f4f4f6
                 52  152 - #f4f4f6
                328  156 - #e05247
                332  156 - #ffffff
                268  160 - #d92d20
                300  160 - #d92d20
                440  160 - #e15b51
                500  160 - #d92e21
                504  160 - #e15b51
                 76  184 - #1c1c1e
                  4  376 - #f4f4f6
                264  376 - #ffffff
                592  436 - #ffffff
                156  576 - #07090b
                 44  580 - #000001
                120  580 - #597c95
                192  580 - #000000
                256  580 - #000000
                308  580 - #597c95
                340  580 - #597c95
                516  592 - #597c95
";
