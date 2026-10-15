use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, ModifiersState, NamedKey, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key, set_record_probe_count},
};

use super::common::{click_at, color_all, quoted, status_style};

/// Typing into a code editor: a click puts the caret, Enter keeps the
/// indent and adds 1 level after a brace, Shift Tab and Tab move the
/// line by 1 level, Backspace takes chars back, and undo and redo walk
/// the steps. The status label shows the lines, each in quotes. The app
/// colors a line again on every change, so the colors stay.
#[view]
struct CodeEditorTyping {
    changes: usize,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorTyping {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(200);
        self.editor.set_text("module part() {\n}");
        color_all(self.editor);
        self.editor.changed.val(move |_| {
            self.changes += 1;
            color_all(self.editor);
        });

        status_style(self.status);
        self.status.place().t(220).lr(10).b(10);
    }
}

impl CodeEditorTyping {
    /// The lines now, also written to the status label with the step.
    fn show(self: Weak<Self>, step: &'static str) -> Vec<String> {
        let lines = from_main(move || {
            let lines = self.editor.lines().to_vec();
            self.status.set_text(format!("{step}\n{}", quoted(&lines)));
            lines
        });
        wait_for_next_frame();
        lines
    }
}

fn command(ch: char) {
    inject_modifiers(ModifiersState::SUPER);
    inject_keys(ch);
    inject_modifiers(ModifiersState::empty());
}

impl ViewTest for CodeEditorTyping {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        click_at(view.editor, CodePos::new(0, 15));
        let editing = from_main(move || view.editor.is_editing());
        ensure!(editing, "a click did not give the editor the keys");
        let lines = view.show("a click after the brace, the caret is there");
        ensure!(
            lines == ["module part() {", "}"],
            "a click changed the text {lines:?}"
        );
        check_colors(CARET)?;

        inject_named_key(NamedKey::Enter);
        inject_keys("cube(1);");
        let lines = view.show("Enter and `cube(1);`, the new line is 1 level in");
        ensure!(
            lines == ["module part() {", "    cube(1);", "}"],
            "Enter and typing gave {lines:?}"
        );
        check_colors(TYPED)?;

        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::Tab);
        inject_modifiers(ModifiersState::empty());
        let lines = view.show("Shift Tab, the line is 1 level out");
        ensure!(lines[1] == "cube(1);", "Shift Tab gave {lines:?}");

        // Tab with no selection puts spaces at the caret, so at the line
        // start it moves the line.
        inject_named_key(NamedKey::Home);
        inject_named_key(NamedKey::Tab);
        inject_named_key(NamedKey::End);
        inject_named_key(NamedKey::Backspace);
        inject_named_key(NamedKey::Backspace);
        let lines = view.show("Home and Tab, then End and 2 Backspaces take `;` and `)`");
        ensure!(lines[1] == "    cube(1", "Tab and Backspace gave {lines:?}");

        // The 2 Backspaces are 1 step, then Tab, Shift Tab, the typing and
        // Enter.
        command('z');
        let lines = view.show("undo, the 2 chars are back");
        ensure!(lines[1] == "    cube(1);", "the first undo gave {lines:?}");
        for _ in 0..4 {
            command('z');
        }
        let lines = view.show("4 more undos, the text is as it was");
        ensure!(lines == ["module part() {", "}"], "the undos gave {lines:?}");

        inject_modifiers(ModifiersState::SUPER | ModifiersState::SHIFT);
        for _ in 0..5 {
            inject_keys('Z');
        }
        inject_modifiers(ModifiersState::empty());
        let (lines, changes, cursor, runs) = from_main(move || {
            let runs: Vec<_> = view.editor.color_runs(1).iter().map(|(range, _)| range.clone()).collect();
            (
                view.editor.lines().to_vec(),
                view.changes,
                view.editor.cursor(),
                runs,
            )
        });
        ensure!(runs == [4..8, 9..10], "line 2 has the color runs {runs:?}");
        let shown = view.show("5 redos, every step is back");
        ensure!(shown == lines);
        ensure!(
            lines == ["module part() {", "    cube(1", "}"],
            "the redos gave {lines:?}"
        );
        ensure!(
            cursor == CodePos::new(1, 10),
            "the caret is at {cursor:?} after the redos"
        );
        ensure!(changes > 0, "the editor fired no change");
        check_colors(REDONE)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const CARET: &str = r"
             316    4 - #ffffff
             592    4 - #ffffff
              72   20 - #cf222e
              76   20 - #e79298
              80   20 - #cf222e
             124   20 - #5b5f63
             168   20 - #ffffff
             456   20 - #ffffff
              56   24 - #eba1a6
              72   24 - #cf222e
              80   24 - #cf222e
             120   24 - #252a30
             124   24 - #5b5f63
             136   24 - #ffffff
             148   24 - #ffffff
              28   48 - #ccd0d6
             384  136 - #ffffff
             100  144 - #ffffff
             540  148 - #ffffff
             208  200 - #ffffff
               4  220 - #ffffff
             412  288 - #ffffff
             568  300 - #ffffff
              48  384 - #383838
              64  384 - #4d4d4d
             140  384 - #4b4b4b
             192  384 - #4d4d4d
              48  388 - #383838
              52  388 - #1f1f1f
              64  388 - #4d4d4d
             104  388 - #676767
             116  388 - #ffffff
             124  388 - #333333
             132  388 - #ffffff
             168  388 - #ffffff
             176  388 - #ffffff
             188  388 - #0a0a0a
             204  388 - #ffffff
             216  388 - #ffffff
             228  388 - #ababab
             240  388 - #ffffff
             276  388 - #a7a7a7
             280  388 - #191919
             288  388 - #ffffff
             292  388 - #343434
              60  400 - #888888
             116  400 - #ffffff
             132  400 - #ffffff
             140  400 - #aeaeae
              64  404 - #2c2c2c
              80  404 - #ffffff
              92  404 - #ffffff
             124  404 - #ffffff
              32  408 - #161616
              40  408 - #ffffff
             108  408 - #747474
             116  408 - #ffffff
             460  440 - #ffffff
             592  456 - #ffffff
             272  512 - #ffffff
               4  580 - #ffffff
             176  592 - #ffffff
             372  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const TYPED: &str = r"
             592    4 - #ffffff
              72   20 - #cf222e
              76   20 - #e79298
              80   20 - #cf222e
             124   20 - #5b5f63
              56   24 - #eba1a6
              72   24 - #cf222e
              80   24 - #cf222e
             120   24 - #252a30
             124   24 - #5b5f63
             136   24 - #ffffff
             148   24 - #ffffff
              92   44 - #c1a9ef
              96   44 - #8250df
             100   44 - #c6aff0
             108   44 - #8453e0
              28   48 - #ccd0d6
             376  132 - #ffffff
             208  192 - #ffffff
             592  284 - #ffffff
              32  372 - #d7d7d7
              92  372 - #848484
             120  372 - #cbcbcb
             148  372 - #9b9b9b
             180  372 - #8e8e8e
             292  372 - #000000
             320  372 - #383838
              76  376 - #cdcdcd
             112  376 - #000000
             120  376 - #cbcbcb
             148  376 - #9b9b9b
             212  376 - #ffffff
             264  376 - #d4d4d4
             292  376 - #000000
             320  376 - #383838
             332  376 - #000000
              36  380 - #343434
              60  380 - #dadada
              92  380 - #848484
             120  380 - #cbcbcb
             148  380 - #9b9b9b
             180  380 - #8e8e8e
             200  380 - #4d4d4d
             264  380 - #d4d4d4
             292  380 - #000000
             320  380 - #383838
             332  380 - #010101
             132  388 - #ffffff
              52  396 - #000000
              64  396 - #2c2c2c
              80  396 - #1a1a1a
              92  396 - #ffffff
             104  396 - #2c2c2c
             120  396 - #ffffff
             104  408 - #ffffff
              52  416 - #ffffff
              68  416 - #ffffff
              80  416 - #ffffff
              92  416 - #353535
             500  440 - #ffffff
               4  588 - #ffffff
             180  592 - #ffffff
             376  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const REDONE: &str = r"
             292    4 - #ffffff
             460    4 - #ffffff
              72   20 - #cf222e
              76   20 - #e79298
              80   20 - #cf222e
             124   20 - #5b5f63
              56   24 - #eba1a6
              72   24 - #cf222e
              80   24 - #cf222e
             120   24 - #252a30
             124   24 - #5b5f63
             136   24 - #ffffff
             148   24 - #ffffff
              92   44 - #c1a9ef
              96   44 - #8250df
             100   44 - #c6aff0
             108   44 - #8453e0
              28   48 - #ccd0d6
             592  112 - #ffffff
             356  136 - #ffffff
             192  196 - #ffffff
               4  212 - #ffffff
             468  216 - #ffffff
             592  272 - #ffffff
             316  284 - #ffffff
             456  352 - #ffffff
              32  372 - #bdbdbd
              60  372 - #6b6b6b
             180  372 - #000000
              48  376 - #ffffff
             112  376 - #000000
             136  376 - #ffffff
             164  376 - #050505
             180  376 - #000000
              40  380 - #1a1a1a
             112  380 - #000000
             136  380 - #ffffff
             164  380 - #050505
             180  380 - #000000
             132  388 - #ffffff
              68  392 - #ffffff
              76  392 - #ffffff
             132  392 - #ffffff
              52  396 - #000000
              64  396 - #2c2c2c
              80  396 - #1a1a1a
              92  396 - #ffffff
             104  396 - #2c2c2c
             120  396 - #ffffff
             124  396 - #ffffff
              76  400 - #ffffff
              60  412 - #ffffff
              92  412 - #353535
              52  416 - #ffffff
              68  416 - #ffffff
              80  416 - #ffffff
              92  416 - #353535
             592  432 - #ffffff
             324  436 - #ffffff
             460  512 - #ffffff
               4  556 - #ffffff
             144  592 - #ffffff
             328  592 - #ffffff
             592  592 - #ffffff
";
