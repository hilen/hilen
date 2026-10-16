use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Label, ModifiersState, NamedKey, Setup, ViewData, ViewFrame, ViewTest, WHITE,
        view,
    },
    ui_test::{
        checkpoint, inject_keys, inject_modifiers, inject_named_key,
        system_input::{press_return, tap, type_text, wait_until},
    },
};

use super::common::{bar_point, click_at, command, command_with, find_state, span, status_style};

const LINES: usize = 120;
/// `target` is on every 30th line from line 8.
const TARGETS: [usize; 4] = [7, 37, 67, 97];

fn long_text() -> String {
    (0..LINES)
        .map(|line| {
            if TARGETS.contains(&line) {
                format!("let target_{line} = {line};")
            } else {
                format!("line {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Next and back in the find bar of a long file. `target` is typed into
/// the bar: 4 matches, 30 lines apart. Enter and Cmd G go to the next
/// match, Shift Enter and Cmd Shift G to the one before, from the first
/// round to the last, and the editor scrolls each one into view under
/// the bar. Escape closes the bar with the match selected. Then a
/// `target` typed into the code is a new match at once.
#[view]
struct CodeEditorFindNext {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorFindNext {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(380);
        self.editor.set_text(&long_text());

        status_style(self.status);
        self.status.place().t(440).lr(10).b(10);
    }
}

impl CodeEditorFindNext {
    /// Checks that match `index` is the current one and that its line is
    /// on screen under the bar, then holds with `step`.
    fn at_match(self: Weak<Self>, index: usize, step: &'static str) -> Result<()> {
        wait_for_next_frame();
        let editor = self.editor;
        let (matches, current) = find_state(editor);
        ensure!(
            current == Some(index),
            "{step}: the current match is {current:?}, not {index}"
        );
        let line = TARGETS[index];
        ensure!(
            matches[index] == span(line, 4, 10),
            "{step}: match {index} is {:?}",
            matches[index]
        );
        let field = bar_point(editor, "query");
        let (top, bottom, row) = from_main(move || {
            let point = editor.point_of(CodePos::new(line, 4)).expect("the line is not laid out");
            let frame = editor.absolute_frame();
            let row = editor.line_height();
            (
                frame.y() + point.y - row / 2.0,
                frame.y() + point.y + row / 2.0,
                frame.max_y(),
            )
        });
        // The row is below the field of the query and inside the editor.
        ensure!(
            top > field.1 && bottom <= row,
            "{step}: line {} is at {top} to {bottom}, the bar ends below {} and the editor at {row}",
            line + 1,
            field.1
        );
        from_main(move || {
            self.status.set_text(step);
        });
        checkpoint(step)
    }
}

fn shift_enter() {
    inject_modifiers(ModifiersState::SHIFT);
    inject_named_key(NamedKey::Enter);
    inject_modifiers(ModifiersState::empty());
}

impl ViewTest for CodeEditorFindNext {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let editor = view.editor;

        click_at(editor, CodePos::new(0, 0));
        command('f');
        let field = bar_point(editor, "query");
        tap(field.0, field.1)?;
        type_text("target")?;
        // A phone starts a field with a capital letter, the case does not
        // count.
        wait_until("target found", move || editor.find_matches().len() == 4)?;
        view.at_match(0, "`target` typed: 4 matches, 1 of 4, on line 8")?;

        press_return()?;
        view.at_match(1, "Enter: 2 of 4, line 38 scrolled into view")?;
        shift_enter();
        view.at_match(0, "Shift Enter: back to 1 of 4")?;
        command('g');
        view.at_match(1, "Cmd G: 2 of 4")?;
        command_with(ModifiersState::SHIFT, 'G');
        view.at_match(0, "Cmd Shift G: 1 of 4")?;
        shift_enter();
        view.at_match(3, "Shift Enter from the first: round to 4 of 4, line 98")?;

        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        let (open, editing, selection) =
            from_main(move || (editor.is_find_open(), editor.is_editing(), editor.selection()));
        ensure!(!open, "Escape did not close the bar");
        ensure!(editing, "the editor did not get the keys back");
        ensure!(
            selection == Some(span(97, 4, 10)),
            "the last match is not selected, the selection is {selection:?}"
        );
        let (matches, _) = find_state(editor);
        ensure!(matches.is_empty(), "a closed bar still marks {matches:?}");
        from_main(move || {
            view.status.set_text("Escape: the bar is gone, the match on line 98 selected");
        });
        checkpoint("Escape: the bar is gone, the match on line 98 selected")?;

        // The selection is the query of the next Cmd F. Line 98 is the
        // last on screen, so the typing goes on the line above it.
        command('f');
        click_at(editor, CodePos::new(96, 7));
        inject_keys(" target");
        wait_until("the typed target is found", move || {
            editor.find_matches().len() == 5
        })?;
        let (matches, _) = find_state(editor);
        ensure!(
            matches[3] == span(96, 8, 14),
            "the typed target is matched at {:?}",
            matches[3]
        );
        from_main(move || {
            view.status.set_text("` target` typed on line 97: a 5th match");
        });
        checkpoint("` target` typed on line 97: a 5th match")?;

        Ok(())
    }
}
