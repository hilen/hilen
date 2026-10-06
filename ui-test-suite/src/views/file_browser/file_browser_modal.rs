use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Button, FileBrowser, FileBrowserControl, FilePick, FilePicker, Label, Setup, TextAlignment,
        TouchStack, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
    window::NamedKey,
};

use super::{picked, sample, tap_control, tap_row};

/// The browser as a dialog. `FilePicker` opens over the page, gives the
/// picked folder on Choose and nothing on Cancel and on Escape.
#[view]
struct FileBrowserModal {
    picker: Weak<FilePicker>,

    #[init]
    open:   Button,
    status: Label,
}

impl FileBrowserModal {
    fn pick(mut self: Weak<Self>) {
        let request = FilePick::folder(sample()).choose_title("Select");
        self.picker = FilePicker::pick(request, move |picked| {
            let text = match picked {
                Some(paths) => format!("picked: {}", paths[0].slash_text()),
                None => "picked nothing".to_string(),
            };
            self.status.set_text(text);
        });
        self.picker.browser().set_open_on_tap(false);
    }
}

impl Setup for FileBrowserModal {
    fn setup(self: Weak<Self>) {
        // The dialog covers the middle of the canvas, the button and
        // the status stay clear of it at the top and the bottom.
        self.open.set_text("Pick a folder").set_text_size(16);
        self.open.set_color("#d9e6ff").set_corner_radius(6);
        self.open.place().t(4).l(20).size(160, 30);
        self.open.on_tap(move || self.pick());

        self.status.set_text("no dialog was open yet").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(4).lr(20).h(30);
    }
}

impl ViewTest for FileBrowserModal {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let open = || inject_touches("100 19 b\n100 19 e");
        let browser = move || -> Weak<FileBrowser> { from_main(move || view.picker.browser()) };
        let status = move || from_main(move || view.status.text().to_string());
        let layers = || from_main(TouchStack::root_name);

        open();
        assert_eq!(layers(), "FilePicker");
        // The dialog fits the 600 canvas with a margin on every side.
        from_main(move || {
            let frame = *view.picker.frame();
            assert_eq!((frame.size.width, frame.size.height), (568.0, 520.0));
        });
        check_colors(CHECK_1)?;

        tap_row(browser(), "Photos")?;
        assert_eq!(picked(browser()), ["Photos"]);
        check_colors(CHECK_2)?;
        tap_control(browser(), FileBrowserControl::Choose);
        assert_eq!(status(), "picked: Disk/Photos");
        assert_eq!(layers(), "Root view");

        open();
        tap_control(browser(), FileBrowserControl::Cancel);
        assert_eq!(status(), "picked nothing");
        assert_eq!(layers(), "Root view");

        // With no row picked Choose gives the open folder.
        open();
        tap_control(browser(), FileBrowserControl::Choose);
        assert_eq!(status(), "picked: Disk");

        open();
        from_main(move || {
            view.status.set_text("dialog open");
        });
        inject_named_key(NamedKey::Escape);
        assert_eq!(status(), "picked nothing");
        assert_eq!(layers(), "Root view");

        Ok(())
    }
}

const CHECK_1: &str = r"
              28    4 - #a3acbf
             376    4 - #435d70
             136   20 - #35383e
             564   64 - #6e6e76
              56   96 - #ceced3
             196  128 - #2f8bf5
             264  128 - #1f1f21
             500  128 - #6e6e76
             204  152 - #63a8f7
             256  156 - #7a7a7c
              32  184 - #8f8f96
             100  184 - #1f1f21
             204  212 - #63a8f7
             592  224 - #435d70
             456  228 - #ffffff
             196  276 - #2f8bf5
             264  276 - #ffffff
             252  308 - #ccccd1
             516  332 - #d2d2d7
              16  352 - #d9d9de
             420  364 - #ffffff
             204  396 - #b4b4bc
             556  396 - #c4c4ca
             300  508 - #ffffff
             492  520 - #0a6cff
              48  536 - #787880
              56  536 - #b0b0b5
             448  536 - #97979a
             540  536 - #0a6cff
              64  580 - #000000
             100  584 - #222f39
             172  584 - #0b0e11
";

const CHECK_2: &str = r"
              28    4 - #a3acbf
             376    4 - #435d70
             136   20 - #35383e
             564   64 - #6e6e76
              56   96 - #ceced3
             236   96 - #828283
             264  128 - #1f1f21
             492  128 - #b6b6ba
             196  152 - #2f8bf5
             256  156 - #7a7a7c
              36  160 - #6e6e76
             100  184 - #1f1f21
             204  212 - #63a8f7
             496  240 - #0a6cff
             204  272 - #63a8f7
             196  276 - #2f8bf5
             264  276 - #ffffff
              20  352 - #f4f4f6
             420  364 - #ffffff
             272  368 - #e2e2e5
             204  372 - #c6c6cc
             556  396 - #c4c4ca
             180  420 - #d9d9de
             300  512 - #f4f4f6
             572  528 - #0a6cff
              48  536 - #787880
              56  536 - #b0b0b5
             448  536 - #97979a
             540  536 - #0a6cff
              64  580 - #000000
             100  584 - #222f39
             172  584 - #0b0e11
";
