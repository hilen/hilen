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
             276    8 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #676768
             376   56 - #c5c8ce
             188   80 - #64a8f7
             192   84 - #2f8bf5
             236   84 - #404041
              16   88 - #dcdce3
             232  116 - #1c1c1e
             188  140 - #c2daff
             512  140 - #0a6cff
              88  144 - #9a9a9c
             372  188 - #0a6cff
             188  200 - #c2daff
             500  208 - #0a6cff
             252  236 - #ffffff
             184  240 - #2f8bf5
             524  264 - #d3d3d6
             444  320 - #85858c
               4  348 - #f4f4f6
             268  356 - #adadae
             556  356 - #d1d1d3
             184  360 - #ffffff
             340  496 - #ffffff
              72  576 - #0c1114
             140  576 - #293945
             180  576 - #000001
              32  580 - #597c95
              56  580 - #597c95
             208  580 - #020203
             464  592 - #597c95
";

const CHECK_2: &str = r"
             304    8 - #d9d9de
             580   20 - #6e6e76
             124   24 - #9e9ea3
             228   56 - #1c1c1e
             444  120 - #85858c
             220  124 - #484849
              76  144 - #1c1c1e
             572  156 - #d6d6d8
             444  240 - #85858c
             236  276 - #ffffff
             548  276 - #bababe
             416  308 - #adadb2
               4  356 - #f4f4f6
             180  396 - #6e6e76
             416  396 - #ffffff
             548  396 - #bababe
             572  516 - #d6d6d8
             260  532 - #0a6cff
             444  540 - #d8e7ff
             416  544 - #0a6cff
             428  544 - #6ea8ff
             548  544 - #7eb2ff
             500  548 - #0a6cff
             184  552 - #4791ff
             188  552 - #4791ff
             132  576 - #020203
              32  580 - #597c95
              56  580 - #597c95
              72  580 - #000000
              88  580 - #435e71
              92  580 - #374d5c
             304  592 - #597c95
";

const CHECK_3: &str = r"
             104   16 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             188   80 - #64a8f7
             192   84 - #2f8bf5
             236   84 - #404041
             232  120 - #3a3a3c
             188  140 - #c2daff
              16  144 - #8f8f96
              88  144 - #9a9a9c
             220  148 - #0a6cff
             500  148 - #0a6cff
             336  156 - #0a6cff
             188  200 - #64a8f7
             252  236 - #ffffff
             184  240 - #2f8bf5
             440  264 - #818188
             524  264 - #d3d3d6
             268  356 - #adadae
             504  356 - #d8d8da
             556  356 - #d1d1d3
             184  360 - #ffffff
               4  396 - #f4f4f6
             340  556 - #ffffff
              72  576 - #0c1114
              84  576 - #000001
              32  580 - #597c95
              56  580 - #597c95
              84  580 - #000001
             592  592 - #597c95
";

const CHECK_4: &str = r"
             272    8 - #d9d9de
             448   20 - #e8e8eb
             580   20 - #6e6e76
             124   24 - #676768
             424   24 - #c3c3c9
             512   52 - #ffffff
             228   56 - #1c1c1e
             376   56 - #c5c8ce
             188   80 - #c2daff
              16   88 - #dcdce3
             240  116 - #9ac2ff
              28  144 - #8f8f96
              88  144 - #9a9a9c
             184  180 - #ffffff
             588  196 - #0a6cff
             252  236 - #0a6cff
             264  264 - #86b6ff
             500  264 - #0a6cff
             416  324 - #72aaff
             236  352 - #9ec5ff
             496  356 - #5197ff
             572  356 - #68a4ff
             376  556 - #ffffff
              72  576 - #0c1114
              88  576 - #3f5869
              32  580 - #597c95
              56  580 - #597c95
              72  580 - #000000
              88  580 - #3f5869
             108  580 - #2d3e4b
             140  580 - #2d3e4b
             592  592 - #597c95
";
