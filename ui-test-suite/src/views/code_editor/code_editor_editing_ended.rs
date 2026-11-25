use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, CodeEditor, CodePos, Color, Label, NamedKey, Setup, TextField, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches, system_input::wait_until},
};

use super::common::{SCAD, click_at, color_all, command, status_style};
use crate::text_points::click;

/// A code editor tells when the keys leave it. A press puts the caret in
/// line 12 of the code. Cmd F gives the keys to the find bar and Escape gives
/// them back to the code: the editing of the file goes on and the count stays
/// at 0. A press on the button under the editor takes the keys, the count
/// goes to 1. With the find bar open again the field under the editor
/// takes the keys from the bar, the count goes to 2. Last an Escape with
/// no selection ends the editing in the code itself, the count goes to 3.
#[view]
struct CodeEditorEditingEnded {
    ended: usize,

    #[init]
    editor: CodeEditor,
    button: Button,
    field:  TextField,
    count:  Label,
    status: Label,
}

impl Setup for CodeEditorEditingEnded {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        self.button.set_text("another view").set_text_size(16);
        self.button.set_color(Color::hex("#D0E4FF"));
        self.button.place().t(400).l(10).size(200, 44);

        self.field.set_placeholder("another field");
        self.field.set_text_size(16);
        self.field.set_color(Color::hex("#EEEEEE"));
        self.field.place().t(400).l(220).r(10).h(44);

        self.count.set_text_size(20);
        self.count.place().t(454).lr(10).h(30);
        self.show_count();
        self.editor.editing_ended.sub(move || {
            self.ended += 1;
            self.show_count();
        });

        status_style(self.status);
        self.status.place().t(494).lr(10).b(10);
    }
}

impl CodeEditorEditingEnded {
    fn show_count(self: Weak<Self>) {
        self.count.set_text(format!("editing ended: {}", self.ended));
    }
}

fn ended(view: Weak<CodeEditorEditingEnded>) -> usize {
    from_main(move || view.ended)
}

fn editing(view: Weak<CodeEditorEditingEnded>) -> bool {
    from_main(move || view.editor.is_editing())
}

/// The check of the editor runs a moment after the keys moved, so a count
/// that must stay is read only after that moment.
fn settle() {
    wait_for_next_frame();
    wait_for_next_frame();
}

fn say(view: Weak<CodeEditorEditingEnded>, text: &'static str) -> Result<()> {
    from_main(move || {
        view.status.set_text(text);
    });
    wait_for_next_frame();
    checkpoint(text)
}

/// A press in the code on `line` that gives the editor the keys. Each
/// press goes to its own line, 2 presses on 1 place in a short time are a
/// double click that selects the word, and Escape would only drop that.
fn press_code(view: Weak<CodeEditorEditingEnded>, line: usize) -> Result<()> {
    click_at(view.editor, CodePos::new(line, 2));
    wait_until("the editor has the keys", move || view.editor.is_editing())
}

fn open_find(view: Weak<CodeEditorEditingEnded>) -> Result<()> {
    command('f');
    wait_until("the find bar is open", move || view.editor.is_find_open())?;
    settle();
    Ok(())
}

impl ViewTest for CodeEditorEditingEnded {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        press_code(view, 11)?;
        ensure!(ended(view) == 0, "a press in the code ended the editing");

        open_find(view)?;
        ensure!(!editing(view), "the code kept the keys under an open find bar");
        ensure!(
            ended(view) == 0,
            "the keys that went into the find bar ended the editing"
        );
        say(view, "Cmd F: the find bar has the keys, the count stays at 0")?;

        inject_named_key(NamedKey::Escape);
        wait_until("the code has the keys back", move || view.editor.is_editing())?;
        settle();
        ensure!(
            ended(view) == 0,
            "the keys that came back from the find bar ended the editing"
        );
        say(
            view,
            "Escape: the bar is gone, the code has the keys again, the count stays at 0",
        )?;

        let button = from_main(move || {
            let frame = view.button.absolute_frame();
            (frame.x() + frame.width() / 2.0, frame.y() + frame.height() / 2.0)
        });
        inject_touches(click(button));
        wait_until("the press on the button ended the editing", move || {
            view.ended == 1
        })?;
        ensure!(
            !editing(view),
            "the code kept the keys after a press on the button"
        );
        say(view, "a press on another view took the keys, the count is 1")?;

        press_code(view, 3)?;
        open_find(view)?;
        ensure!(ended(view) == 1, "the find bar ended the editing a second time");
        from_main(move || view.field.focus());
        wait_until("the field took the keys from the find bar", move || {
            view.ended == 2
        })?;
        settle();
        ensure!(
            ended(view) == 2,
            "the field ended the editing {} times",
            ended(view) - 1
        );
        say(
            view,
            "the find bar was open, another field took its keys, the count is 2",
        )?;

        from_main(move || {
            let mut editor = view.editor;
            editor.close_find();
        });
        press_code(view, 1)?;
        settle();
        ensure!(ended(view) == 2, "a press back in the code ended the editing");
        inject_named_key(NamedKey::Escape);
        wait_until("Escape ended the editing", move || view.ended == 3)?;
        ensure!(!editing(view), "the code kept the keys after Escape");
        say(
            view,
            "Escape with no selection ended the editing in the code, the count is 3",
        )?;
        Ok(())
    }
}
