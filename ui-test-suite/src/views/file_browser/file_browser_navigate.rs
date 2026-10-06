use anyhow::{Result, anyhow};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Label, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

use super::{double_tap_row, names, picked, sample, tap_at, tap_control};

/// Moving around: a double tap opens a folder, a crumb and a place jump,
/// back and forward walk the history, up goes to the parent and picks
/// the folder it came from.
#[view]
struct FileBrowserNavigate {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserNavigate {
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

impl ViewTest for FileBrowserNavigate {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let crumbs = move || from_main(move || browser.crumb_titles());
        let history = move || from_main(move || (browser.can_go_back(), browser.can_go_forward()));

        double_tap_row(browser, "Photos")?;
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);
        assert_eq!(crumbs(), ["Disk", "Photos"]);
        assert_eq!(history(), (true, false));
        check_colors(CHECK_1)?;

        double_tap_row(browser, "Trip")?;
        assert_eq!(names(browser), ["beach.png"]);
        assert_eq!(crumbs(), ["Disk", "Photos", "Trip"]);
        checkpoint("2 folders deep, 3 crumbs")?;

        // A crumb opens its folder and is a new step of the history.
        let disk = from_main(move || browser.crumb_center("Disk")).ok_or_else(|| anyhow!("no Disk crumb"))?;
        tap_at(disk);
        assert_eq!(crumbs(), ["Disk"]);
        assert_eq!(history(), (true, false));
        checkpoint("the Disk crumb went to the root")?;

        tap_control(browser, FileBrowserControl::Back);
        assert_eq!(crumbs(), ["Disk", "Photos", "Trip"]);
        assert_eq!(history(), (true, true));
        tap_control(browser, FileBrowserControl::Back);
        assert_eq!(crumbs(), ["Disk", "Photos"]);
        tap_control(browser, FileBrowserControl::Forward);
        assert_eq!(crumbs(), ["Disk", "Photos", "Trip"]);
        checkpoint("back twice, forward once, in Trip again")?;

        // Up picks the folder it left, so the eye finds it.
        tap_control(browser, FileBrowserControl::Up);
        assert_eq!(crumbs(), ["Disk", "Photos"]);
        assert_eq!(picked(browser), ["Trip"]);
        check_colors(CHECK_2)?;

        let documents = from_main(move || browser.place_center("Documents"))
            .ok_or_else(|| anyhow!("no Documents place"))?;
        tap_at(documents);
        assert_eq!(names(browser), ["budget.xlsx", "notes.txt", "report.pdf"]);
        assert_eq!(view.status.text(), "open folder: Disk/Documents");
        check_colors(CHECK_3)?;

        // A file does not open like a folder, it fires `opened`.
        double_tap_row(browser, "notes.txt")?;
        assert_eq!(crumbs(), ["Disk", "Documents"]);

        Ok(())
    }
}

const CHECK_1: &str = r"
             308    8 - #d9d9de
             500    8 - #d9d9de
             172   20 - #a3a3a4
             388   20 - #ffffff
             580   20 - #6e6e76
             124   24 - #9e9ea3
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             436   56 - #7c7c83
             188   80 - #64a8f7
              52   84 - #f4f4f6
             192   84 - #2f8bf5
             224   84 - #bababa
             192   88 - #2f8bf5
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
             504  140 - #85858c
              16  144 - #8f8f96
              84  144 - #cdcdcf
             188  148 - #ffffff
             320  356 - #ffffff
             592  528 - #ffffff
             184  576 - #000000
              32  580 - #597c95
              56  580 - #000000
              84  580 - #151d22
             104  580 - #364b5a
             120  580 - #597c95
             140  580 - #2d3e4b
             392  592 - #597c95
             504  592 - #597c95
";

const CHECK_2: &str = r"
             288    8 - #d9d9de
             104   16 - #d9d9de
             172   20 - #a3a3a4
             580   20 - #6e6e76
             436   56 - #7c7c83
             188   80 - #c2daff
             220   80 - #0a6cff
             512   80 - #0a6cff
              52   84 - #f4f4f6
             216   84 - #81b3ff
             224   84 - #5599ff
             332   96 - #0a6cff
             392   96 - #0a6cff
             504  116 - #b7b7bb
             556  116 - #d4d4d7
             444  140 - #85858c
             504  140 - #85858c
              16  144 - #8f8f96
              84  144 - #cdcdcf
             232  144 - #1c1c1e
             188  148 - #ffffff
             336  360 - #ffffff
             592  528 - #ffffff
             184  576 - #000000
              32  580 - #597c95
              56  580 - #000000
              84  580 - #151d22
             104  580 - #364b5a
             120  580 - #597c95
             140  580 - #2d3e4b
             392  592 - #597c95
             504  592 - #597c95
";

const CHECK_3: &str = r"
             288    8 - #d9d9de
             104   16 - #d9d9de
             388   20 - #ffffff
             448   20 - #e8e8eb
             580   20 - #6e6e76
             196   24 - #ffffff
              20   80 - #6e6e76
             444   80 - #85858c
              68   84 - #8e8e90
             220   84 - #434345
             504   84 - #b7b7bb
             252  116 - #ffffff
             548  116 - #bababe
              16  136 - #dcdce3
             444  140 - #85858c
             100  144 - #dcdce3
             244  144 - #1c1c1e
             544  144 - #949499
             180  148 - #6e6e76
             424  148 - #adadb2
             304  352 - #ffffff
             592  528 - #ffffff
              84  576 - #151d22
             140  576 - #2d3e4b
             224  576 - #000000
              44  580 - #000001
              56  580 - #000000
             120  580 - #597c95
             160  580 - #597c95
             184  580 - #23303a
             392  592 - #597c95
             504  592 - #597c95
";
