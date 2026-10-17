use anyhow::Result;
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{FileBrowser, Label, ModifiersState, Setup, TextAlignment, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_key, inject_keys, inject_modifiers, inject_named_key},
    window::NamedKey,
};

use super::{names, picked, sample, tap_row};

/// The keyboard: after a press on the list the arrows move the picked
/// row, Shift adds rows, Enter opens, End and Home jump and scroll, typed
/// letters jump to a name, Backspace goes up and Cmd A picks all.
#[view]
struct FileBrowserKeys {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserKeys {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser.set_open_on_tap(false).set_source(sample());
        self.browser.selection_changed.val(move |paths| {
            let names: Vec<&str> = paths.iter().map(FilePath::name).collect();
            // A long list would not fit the line.
            if names.len() > 3 {
                self.status.set_text(format!("picked: {} entries", names.len()));
            } else {
                self.status.set_text(format!("picked: {}", names.join(", ")));
            }
        });

        self.status.set_text("picked:").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

impl ViewTest for FileBrowserKeys {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let on_screen = move |name: &'static str| from_main(move || browser.row_center(name).is_some());

        // Before any press on the list the keys are not its own.
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), Vec::<String>::new());

        tap_row(browser, "Documents")?;
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), ["Empty"]);
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), ["Many"]);

        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::ArrowDown);
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), ["Many", "Music", "Photos"]);
        check_colors(CHECK_1)?;

        inject_modifiers(ModifiersState::empty());
        inject_named_key(NamedKey::ArrowUp);
        inject_named_key(NamedKey::ArrowUp);
        assert_eq!(picked(browser), ["Many"]);

        inject_named_key(NamedKey::Enter);
        assert_eq!(names(browser).len(), 40);
        assert_eq!(picked(browser), Vec::<String>::new());
        checkpoint("Enter opened Many, 40 files, none picked")?;

        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser), ["file-01.txt"]);
        assert!(!on_screen("file-40.txt"));

        inject_named_key(NamedKey::End);
        assert_eq!(picked(browser), ["file-40.txt"]);
        assert!(on_screen("file-40.txt"));
        assert!(!on_screen("file-01.txt"));
        check_colors(CHECK_2)?;

        inject_named_key(NamedKey::Home);
        assert_eq!(picked(browser), ["file-01.txt"]);
        assert!(on_screen("file-01.txt"));

        // The letters typed in a row make one name to jump to.
        inject_keys("file-2");
        assert_eq!(picked(browser), ["file-20.txt"]);
        assert!(on_screen("file-20.txt"));
        checkpoint("typing file-2 jumped to file-20.txt")?;

        inject_named_key(NamedKey::Backspace);
        assert_eq!(from_main(move || browser.crumb_titles()), ["Disk"]);
        assert_eq!(picked(browser), ["Many"]);
        check_colors(CHECK_3)?;

        inject_modifiers(ModifiersState::SUPER);
        inject_key('a');
        inject_modifiers(ModifiersState::empty());
        assert_eq!(picked(browser).len(), 10);
        check_colors(CHECK_4)?;

        // Left has no use in a list, the key focus ring gets the keys
        // back with it and the picked rows stay.
        inject_named_key(NamedKey::ArrowLeft);
        inject_named_key(NamedKey::ArrowDown);
        assert_eq!(picked(browser).len(), 10);

        Ok(())
    }
}

const CHECK_1: &str = r"
             316    8 - #d9d9de
             424    8 - #d9d9de
             124   24 - #676768
             516   56 - #6e6e76
              36   60 - #f4f4f6
             236   84 - #404041
             228  116 - #c4c4c5
             504  140 - #0a6cff
              76  144 - #1c1c1e
              88  144 - #9a9a9c
             364  148 - #0a6cff
             184  176 - #0a6cff
             588  180 - #0a6cff
             252  236 - #ffffff
             444  260 - #85858c
             524  264 - #d3d3d6
             544  264 - #d8d8da
             184  292 - #ffffff
             416  324 - #c2c2c5
             532  324 - #99999e
               4  340 - #f4f4f6
             276  352 - #1c1c1e
             212  356 - #e8e8e8
             496  356 - #d5d5d7
             556  356 - #d1d1d3
             440  556 - #ffffff
              84  576 - #000001
             208  576 - #020203
              32  580 - #597c95
             144  580 - #344958
             328  592 - #597c95
             592  592 - #597c95
";

const CHECK_2: &str = r"
             356    4 - #f4f4f6
             284    8 - #d9d9de
             484    8 - #d9d9de
             124   24 - #9e9ea3
             228   56 - #1c1c1e
              68   84 - #8e8e90
             252   96 - #4d4d4f
             572   96 - #d6d6d8
              76  144 - #1c1c1e
             440  184 - #ffffff
             548  184 - #bababe
             236  276 - #ffffff
             548  304 - #bababe
               4  360 - #f4f4f6
             424  364 - #6f6f77
             244  396 - #ffffff
             544  424 - #dcdcde
             576  516 - #7f7f86
             208  532 - #0a6cff
             316  532 - #0a6cff
             444  540 - #d8e7ff
             416  544 - #0a6cff
             428  544 - #6ea8ff
             544  544 - #4690ff
             548  544 - #7eb2ff
             500  548 - #0a6cff
             132  576 - #020203
              72  580 - #000000
              88  580 - #435e71
              92  580 - #374d5c
             276  592 - #597c95
             356  592 - #597c95
";

const CHECK_3: &str = r"
             316    8 - #d9d9de
             124   24 - #676768
             424   24 - #c3c3c9
             516   56 - #6e6e76
               8   76 - #dcdce3
             236   84 - #404041
             240   84 - #1c1c1e
             232  120 - #3a3a3c
             504  140 - #0a6cff
              88  144 - #9a9a9c
             184  144 - #0a6cff
             368  156 - #0a6cff
             588  156 - #0a6cff
             188  204 - #ffffff
             252  236 - #ffffff
             444  260 - #85858c
             220  264 - #1c1c1e
             524  264 - #d3d3d6
             544  264 - #d8d8da
             532  324 - #99999e
               4  336 - #f4f4f6
             212  356 - #e8e8e8
             268  356 - #adadae
             420  356 - #ffffff
             496  356 - #d5d5d7
             556  356 - #d1d1d3
             364  556 - #ffffff
              84  576 - #000001
              32  580 - #597c95
              72  580 - #000000
             224  592 - #597c95
             592  592 - #597c95
";

const CHECK_4: &str = r"
             320    8 - #d9d9de
             116   20 - #bcbcbd
             124   24 - #676768
             424   24 - #c3c3c9
             512   52 - #ffffff
             228   56 - #1c1c1e
              68   84 - #818186
              24   88 - #dcdce3
             156   96 - #dcdce3
             232  120 - #deebff
              88  144 - #9a9a9c
             368  148 - #0a6cff
             504  176 - #0a6cff
             588  196 - #0a6cff
             252  236 - #0a6cff
             444  260 - #d8e7ff
             500  264 - #0a6cff
             232  296 - #aecfff
             416  324 - #72aaff
             236  352 - #9ec5ff
               4  356 - #f4f4f6
             496  356 - #5197ff
             572  356 - #68a4ff
             376  556 - #ffffff
              88  576 - #3f5869
              32  580 - #597c95
              72  580 - #000000
             108  580 - #2d3e4b
             124  580 - #0d1215
             140  580 - #2d3e4b
             260  592 - #597c95
             592  592 - #597c95
";
