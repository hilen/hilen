use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, CodeEditor, CodePos, FindOptions, Label, Setup, Theme, ThemeMode, UIManager, ViewData,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

use super::common::{SCAD, click_at, click_bar, color_all, command, find_state, span, status_style};

/// The find bar of a code editor. `plate` is selected and Cmd F opens the
/// bar over the top of the editor with it as the query: the 3 matches
/// get a light yellow background, the current one on line 4 a stronger
/// one, and the bar says "2 of 3". The toggles for the case, whole words
/// and a regular expression change what matches. Then the look with 2
/// toggles on, and the same in the dark theme.
#[view]
struct CodeEditorFind {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorFind {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(440).lr(10).b(10);
    }
}

impl CodeEditorFind {
    /// The step on the status label, with no field and no editor that
    /// has the keys, so no caret is in a checked picture.
    fn show(self: Weak<Self>, step: &'static str) {
        from_main(move || {
            UIManager::unselect_view();
            self.status.set_text(step);
        });
        wait_for_next_frame();
    }

    /// Holds with `step` on the status label.
    fn say(self: Weak<Self>, step: &'static str) -> Result<()> {
        from_main(move || {
            self.status.set_text(step);
        });
        checkpoint(step)
    }

    /// A new query in the bar, the way an app puts one in.
    fn query(self: Weak<Self>, query: &'static str) -> usize {
        let mut editor = self.editor;
        from_main(move || editor.show_find(query));
        wait_for_next_frame();
        find_state(editor).0.len()
    }
}

impl ViewTest for CodeEditorFind {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let mut editor = view.editor;

        click_at(editor, CodePos::new(3, 7));
        from_main(move || editor.select(CodePos::new(3, 7), CodePos::new(3, 12)));
        command('f');
        let open = from_main(move || editor.is_find_open());
        ensure!(open, "Cmd F did not open the find bar");
        let (matches, current) = find_state(editor);
        ensure!(
            matches == [span(0, 5, 10), span(3, 7, 12), span(11, 0, 5)],
            "plate matched {matches:?}"
        );
        ensure!(current == Some(1), "the current match is {current:?}");
        view.show(
            "Cmd F with `plate` selected: 3 matches lit, the one on line 4 stronger, the bar says 2 of 3",
        );
        check_colors(LIGHT)?;

        let found = view.query("Plate");
        ensure!(found == 3, "Plate with the case off found {found}");
        click_bar(editor, "case");
        let (matches, _) = find_state(editor);
        ensure!(matches.is_empty(), "Plate with the case on found {matches:?}");
        let case = from_main(move || editor.find_options().case_sensitive);
        ensure!(case, "the case toggle did not turn on");
        view.say("Aa on: `Plate` with a capital finds nothing")?;
        click_bar(editor, "case");

        let found = view.query("plat");
        ensure!(found == 3, "plat found {found}");
        click_bar(editor, "word");
        let (matches, _) = find_state(editor);
        ensure!(matches.is_empty(), "plat as a whole word found {matches:?}");
        view.say("W on: `plat` is no whole word, nothing found")?;
        click_bar(editor, "word");

        click_bar(editor, "regex");
        let found = view.query(r"\w+\(");
        ensure!(
            found == 7,
            r"\w+\( found {found}, not the 7 names before a bracket"
        );
        view.say(".* on: 7 names right before a bracket")?;
        click_bar(editor, "regex");

        let options = from_main(move || editor.find_options());
        ensure!(
            options == FindOptions::default(),
            "the toggles stayed on: {options:?}"
        );
        let api = from_main(move || editor.find("size", FindOptions::default()));
        ensure!(
            api == Ok(vec![span(1, 0, 4), span(5, 13, 17)]),
            "find of size gave {api:?}"
        );
        let regex = FindOptions {
            regex: true,
            ..FindOptions::default()
        };
        let bad = from_main(move || editor.find("(", regex).is_err());
        ensure!(bad, "a regex that does not parse gave no error");

        let found = view.query("plate");
        ensure!(found == 3, "plate found {found} after the toggles");
        click_bar(editor, "case");
        click_bar(editor, "word");
        let found = find_state(editor).0.len();
        ensure!(found == 3, "plate with Aa and W found {found}");
        view.show("Aa and W on: `plate` still finds 3 whole words, the 2 toggles lit");
        check_colors(TOGGLES)?;

        from_main(move || {
            Theme::set_mode(ThemeMode::Dark);
            view.set_color(BLACK);
            view.status.set_text_color(WHITE);
        });
        view.show("the same in the dark theme");
        check_colors(DARK)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LIGHT: &str = r"
             320    4 - #ffffff
             460   68 - #e6eaee
             516   68 - #555a60
             544   68 - #555a60
             112   88 - #fff1a8
              92   92 - #fff1a8
             120   96 - #fff1a8
             168  100 - #747c86
             184  100 - #6f7881
             200  100 - #747c86
             104  104 - #fff1a8
             128  104 - #fff1a8
              76  120 - #282d33
              28  124 - #ccd1d6
             116  156 - #f9c513
             132  156 - #f9c513
             144  156 - #f9c513
              72  160 - #cf222e
             140  160 - #f9c513
              56  164 - #eba1a6
              76  164 - #e79298
              80  164 - #cf222e
              92  164 - #d02632
             108  164 - #c4916e
             120  164 - #f9c513
             136  164 - #f9c513
             132  168 - #f9c513
             172  184 - #ffffff
              88  188 - #8250df
             124  188 - #a17ce7
             140  188 - #c7b0f1
             140  208 - #8453e0
              28  224 - #bbc1c8
             124  228 - #a17ce7
             140  232 - #c7b0f1
             168  232 - #8251df
             180  252 - #c7b0f1
             204  252 - #a17ce7
             284  252 - #5b5f63
             316  252 - #ffffff
             260  256 - #9cbadf
              32  272 - #8d96a0
             592  288 - #ffffff
              52  336 - #fff1a8
              88  336 - #fff1a8
              72  340 - #8251df
             444  344 - #ffffff
             120  360 - #24292f
             172  364 - #ffffff
              68  500 - #0b0b0b
             316  504 - #000000
             332  504 - #b3b3b3
             440  504 - #000000
             540  504 - #989898
              44  508 - #dddddd
             184  508 - #ffffff
             252  508 - #000000
             284  508 - #8d8d8d
             332  508 - #b3b3b3
             364  508 - #747474
             400  508 - #d9d9d9
             476  508 - #343434
             116  512 - #323232
              68  528 - #9d9d9d
";

/// Recorded with `--record-colors`.
const TOGGLES: &str = r"
             360   56 - #b6e3ff
             332   64 - #b6e3ff
             516   64 - #2e3338
             460   68 - #e6eaee
             544   68 - #555a60
             368   72 - #b6e3ff
             380   76 - #b6e3ff
             348   80 - #b6e3ff
             112   88 - #fff1a8
             128   88 - #fff1a8
              92   92 - #fff1a8
             120  100 - #fff1a8
             168  100 - #747c86
             196  100 - #bec2c6
             108  104 - #fff1a8
             120  116 - #b1c8e6
              28  124 - #ccd1d6
             124  156 - #f9c513
             136  156 - #f9c513
             144  156 - #f9c513
             108  160 - #f9c513
              56  164 - #eba1a6
              72  164 - #cf232f
              76  164 - #e79298
              80  164 - #cf222e
              92  164 - #d02632
             108  164 - #c4916e
             120  164 - #f9c513
             140  164 - #8452db
             132  168 - #f9c513
              88  188 - #8250df
             124  188 - #a17ce7
             124  208 - #c1a9ef
             180  208 - #282d33
              28  224 - #bbc1c8
             292  228 - #ffffff
             124  232 - #a17ce7
             140  232 - #c7b0f1
             168  232 - #8251df
             180  252 - #c7b0f1
             204  252 - #a17ce7
             220  252 - #9c9ea1
             316  252 - #ffffff
             260  256 - #9cbadf
              32  272 - #8d96a0
             592  304 - #ffffff
              52  336 - #fff1a8
              72  336 - #fff1a8
              84  340 - #8453de
             440  356 - #ffffff
              60  364 - #9c9ea1
             120  364 - #24292f
             172  364 - #ffffff
              64  516 - #191919
             108  516 - #010101
             148  516 - #121212
             212  516 - #000000
             272  516 - #191919
             292  516 - #525252
             352  516 - #848484
             408  516 - #ffffff
             452  516 - #525252
             380  520 - #808080
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DARK: &str = r"
              12   52 - #21262d
             328   56 - #1f6feb
             376   56 - #1f6feb
             352   60 - #1f6feb
              52   64 - #0d1117
             332   64 - #1f6feb
             516   64 - #dde4e9
             544   68 - #b5bbc2
             336   72 - #1f6feb
             368   72 - #1f6feb
             444   72 - #40464f
             588   84 - #21262d
              92   88 - #4d3d00
             124   88 - #4d3d00
             100   96 - #4d3d00
             112  100 - #8b949d
             168  100 - #868f99
             200  100 - #868f99
             128  104 - #4d3d00
              76  120 - #e2e9ef
             124  156 - #9e6a03
             136  156 - #9e6a03
             144  156 - #9e6a03
             108  160 - #9e6a03
              56  164 - #733e3e
              72  164 - #fe7b72
              76  164 - #854544
              80  164 - #ff7b72
              92  164 - #fa7970
             108  164 - #b58674
             120  164 - #9e6a03
             140  164 - #d1a7fa
             132  168 - #9e6a03
             124  184 - #a182c5
             172  184 - #0d1117
              88  188 - #d2a8ff
             140  188 - #665580
             140  208 - #cea5fa
             180  208 - #e2e9ef
              28  224 - #474d57
             124  228 - #a182c5
             168  232 - #d1a7fe
             180  252 - #665580
             204  252 - #a182c5
             284  252 - #b0b6bc
             316  252 - #0d1117
              32  272 - #6d7580
             584  324 - #0d1117
              52  336 - #4d3d00
              72  336 - #4d3d00
              88  336 - #4d3d00
              52  340 - #886d72
              52  360 - #787e84
             120  364 - #e6edf3
             172  364 - #0d1117
              12  428 - #161b22
             420  460 - #000000
              32  512 - #b2b2b2
              68  516 - #545454
              76  516 - #ffffff
             128  516 - #ffffff
             176  516 - #fafafa
             216  516 - #adadad
             592  592 - #000000
";
