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
             296    8 - #d9d9de
             400    8 - #d9d9de
             124   24 - #9e9ea3
             176   24 - #d6d6d6
             452   24 - #f9f9f9
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
              68   84 - #8e8e90
             224   84 - #bababa
             140  100 - #dcdce3
             504  116 - #b7b7bb
             556  116 - #d4d4d7
              52  140 - #f4f4f6
              88  144 - #9a9a9c
             232  144 - #1c1c1e
             412  144 - #dcdcde
             188  148 - #ffffff
               4  360 - #f4f4f6
             316  364 - #ffffff
             592  552 - #ffffff
             280  560 - #597c95
             140  576 - #2d3e4b
             156  576 - #07090b
              32  580 - #597c95
              56  580 - #000000
              80  580 - #000000
             104  580 - #364b5a
             128  580 - #2e414e
             184  580 - #000000
             376  592 - #597c95
             488  592 - #597c95
";

const CHECK_2: &str = r"
             276    8 - #d9d9de
             124   24 - #9e9ea3
             176   24 - #d6d6d6
             452   24 - #f9f9f9
             228   56 - #1c1c1e
             436   56 - #7c7c83
             516   56 - #6e6e76
             368   72 - #0a6cff
             220   80 - #0a6cff
              52   84 - #f4f4f6
             184   84 - #0a6cff
             216   84 - #81b3ff
             224   84 - #5599ff
             296   96 - #0a6cff
             228  116 - #a7a7a8
             504  116 - #b7b7bb
             556  116 - #d4d4d7
               8  124 - #dcdce3
              88  144 - #9a9a9c
             232  144 - #1c1c1e
             412  144 - #dcdcde
             188  148 - #ffffff
             316  364 - #ffffff
             592  552 - #ffffff
             156  576 - #07090b
              32  580 - #597c95
              80  580 - #000000
             104  580 - #364b5a
             128  580 - #2e414e
             184  580 - #000000
             376  592 - #597c95
             488  592 - #597c95
";

const CHECK_3: &str = r"
             320    8 - #d9d9de
             116   20 - #d4d4d7
             168   20 - #ffffff
             388   20 - #ffffff
             448   20 - #e8e8eb
             216   24 - #a9a9a9
             516   56 - #6e6e76
             444   80 - #85858c
              68   84 - #8e8e90
             220   84 - #434345
             424   84 - #6e6e76
             504   84 - #b7b7bb
             252  116 - #ffffff
             548  116 - #bababe
             572  116 - #d6d6d8
             444  140 - #85858c
              76  144 - #1c1c1e
             244  144 - #1c1c1e
             512  144 - #6e6e76
             544  144 - #949499
             156  152 - #dcdce3
             320  352 - #ffffff
             592  496 - #ffffff
             140  576 - #2d3e4b
             224  576 - #000000
              44  580 - #000001
              84  580 - #151d22
             120  580 - #597c95
             160  580 - #597c95
             192  580 - #000000
             420  592 - #597c95
             532  592 - #597c95
";
