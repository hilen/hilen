use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Label, ModifiersState, NamedKey, Setup, UIManager, ViewData, ViewTest, WHITE,
        view,
    },
    ui_test::{
        check_colors, checkpoint, inject_modifiers, inject_named_key, set_record_probe_count,
        system_input::{press_return, tap, type_text, wait_until},
    },
};

use super::common::{
    SCAD, bar_point, click_at, color_all, command, find_state, replace_keys, span, status_style,
};

/// Replace in the find bar of a code editor. `size` is selected and Cmd
/// Alt F on a Mac, Ctrl H elsewhere, opens the bar with the replace row.
/// `dims` is typed: Enter replaces the current match, Cmd Enter all the
/// rest. Escape gives the editor the keys, the first Cmd Z takes back the
/// Replace All as 1 step and the second the first replace.
#[view]
struct CodeEditorReplace {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorReplace {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.changed.val(move |_| color_all(self.editor));

        status_style(self.status);
        self.status.place().t(440).lr(10).b(10);
    }
}

impl CodeEditorReplace {
    /// The lines in lower case, a phone types `Dims` with a capital.
    fn lines(self: Weak<Self>) -> Vec<String> {
        let editor = self.editor;
        from_main(move || editor.lines().iter().map(|line| line.to_lowercase()).collect())
    }

    fn hold(self: Weak<Self>, step: &'static str) -> Result<()> {
        from_main(move || {
            self.status.set_text(step);
        });
        checkpoint(step)
    }
}

impl ViewTest for CodeEditorReplace {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let mut editor = view.editor;

        click_at(editor, CodePos::new(1, 0));
        from_main(move || editor.select(CodePos::new(1, 0), CodePos::new(1, 4)));
        replace_keys();
        let open = from_main(move || editor.is_find_open());
        ensure!(open, "the replace keys did not open the find bar");
        let (matches, current) = find_state(editor);
        ensure!(
            matches == [span(1, 0, 4), span(5, 13, 17)],
            "size matched {matches:?}"
        );
        ensure!(current == Some(0), "the current match is {current:?}");
        // No field has the keys in the checked picture, so no caret.
        from_main(move || {
            UIManager::unselect_view();
            view.status
                .set_text("the replace keys with `size` selected: the replace row, 1 of 2");
        });
        wait_for_next_frame();
        check_colors(REPLACE_ROW)?;

        let field = bar_point(editor, "replacement");
        tap(field.0, field.1)?;
        type_text("dims")?;
        press_return()?;
        wait_until("the first size is replaced", move || {
            editor.lines()[1] != "size = [40, 30, 10];"
        })?;
        let lines = view.lines();
        ensure!(
            lines[1] == "dims = [40, 30, 10];",
            "Enter replaced line 2 to {:?}",
            lines[1]
        );
        let (matches, current) = find_state(editor);
        ensure!(
            matches == [span(5, 13, 17)] && current == Some(0),
            "after the replace the matches are {matches:?}, the current one {current:?}"
        );
        view.hold("`dims` and Enter: line 2 replaced, 1 of 1 left on line 6")?;

        inject_modifiers(ModifiersState::SUPER);
        inject_named_key(NamedKey::Enter);
        inject_modifiers(ModifiersState::empty());
        wait_for_next_frame();
        let lines = view.lines();
        ensure!(
            lines[5] == "        cube(dims);",
            "Replace All gave line 6 {:?}",
            lines[5]
        );
        let (matches, _) = find_state(editor);
        ensure!(matches.is_empty(), "size is still found at {matches:?}");
        view.hold("Cmd Enter: Replace All, no size left")?;

        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        let (open, editing) = from_main(move || (editor.is_find_open(), editor.is_editing()));
        ensure!(
            !open && editing,
            "Escape left the bar open {open} or the editor without keys {editing}"
        );

        command('z');
        let lines = view.lines();
        ensure!(
            lines[1] == "dims = [40, 30, 10];" && lines[5] == "        cube(size);",
            "the first undo gave {:?} and {:?}",
            lines[1],
            lines[5]
        );
        view.hold("Cmd Z: the Replace All is back as 1 step, line 2 still dims")?;

        command('z');
        let text = from_main(move || editor.text());
        ensure!(
            text == SCAD,
            "the second undo did not give the file back:\n{text}"
        );
        view.hold("Cmd Z again: the file as it was")?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const REPLACE_ROW: &str = r"
             296    4 - #ffffff
             444   64 - #dee3e8
             444   68 - #dee3e8
             460   68 - #e6eaee
              52   72 - #75787c
             468   72 - #c4cad1
              48  100 - #8c959f
             520  100 - #eaeef2
             568  100 - #373c41
             500  104 - #adb1b6
             160  128 - #ffffff
             200  132 - #747c86
              52  144 - #f9c513
              80  144 - #f9c513
              72  148 - #f9c513
             120  148 - #b1c8e6
              60  152 - #f9c513
              64  152 - #f9c513
              28  156 - #ccd1d6
              72  192 - #cf222e
              56  196 - #eba1a6
              76  196 - #e79298
              80  196 - #cf222e
              92  196 - #d02632
             140  196 - #8453e0
              88  216 - #8250df
             124  220 - #a17ce7
             156  232 - #fff1a8
             184  232 - #fff1a8
             176  236 - #fff1a8
             124  240 - #c1a9ef
             164  240 - #fff1a8
             168  240 - #fff1a8
              28  256 - #bbc1c8
             124  260 - #a17ce7
             140  264 - #c7b0f1
             168  264 - #8251df
             180  284 - #c7b0f1
             204  284 - #a17ce7
             284  284 - #5b5f63
             316  284 - #ffffff
             260  288 - #9cbadf
              32  304 - #8d96a0
             592  304 - #ffffff
              72  372 - #8251df
             148  392 - #5b5f63
              56  396 - #24292f
             104  396 - #272c32
              32  512 - #4d4d4d
             280  512 - #eaeaea
             340  512 - #000000
             408  512 - #8d8d8d
              68  516 - #000000
             100  516 - #525252
             164  516 - #ffffff
             260  516 - #1e1e1e
             280  516 - #eaeaea
             300  516 - #999999
             372  516 - #e6e6e6
             384  516 - #000000
             408  516 - #8d8d8d
             200  520 - #808080
             440  520 - #808080
             592  592 - #ffffff
";
