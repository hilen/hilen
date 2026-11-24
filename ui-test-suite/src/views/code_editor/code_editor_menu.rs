use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, ContextMenu, Focus, Label, MenuItem, NamedKey, Point, Setup, ViewData, ViewTest,
        WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_named_key},
};

use super::common::{SCAD, click_at, color_all, screen_point, status_style};

/// The line with `size = [40, 30, 10];`, the caret goes behind `size`.
const LINE: usize = 1;
const BYTE: usize = 4;

/// A menu over a code editor that has the keys, like the fixes an app
/// offers at the caret. While the menu is open the keys are its own: an
/// arrow walks its rows and moves no caret, a letter changes no code,
/// Enter picks the row under the ring. A pick is a tap on a row, which
/// takes the keys from the editor, so the action of a row gives them
/// back, the way the rows of the editor's own menu do. Escape closes a
/// menu with no tap, and the editor has the keys as before.
#[view]
struct CodeEditorMenu {
    picked: Option<&'static str>,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorMenu {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

/// Opens a menu with 2 rows under the line of the caret.
fn open_menu(view: Weak<CodeEditorMenu>) {
    from_main(move || {
        let mut view = view;
        let (x, y) = screen_point(view.editor, CodePos::new(LINE, BYTE));
        let items = ["first", "second"]
            .into_iter()
            .map(|name| {
                MenuItem::new(name, move || {
                    view.picked = Some(name);
                    view.editor.focus();
                })
            })
            .collect();
        ContextMenu::show(items, Point::new(x, y + view.editor.line_height()));
    });
    wait_for_next_frame();
}

/// The text, the caret, whether the editor has the keys and whether a menu
/// is open.
fn state(view: Weak<CodeEditorMenu>) -> (String, CodePos, bool, bool) {
    from_main(move || {
        (
            view.editor.text(),
            view.editor.cursor(),
            view.editor.is_editing(),
            ContextMenu::open().is_ok(),
        )
    })
}

fn say(view: Weak<CodeEditorMenu>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorMenu {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let caret = CodePos::new(LINE, BYTE);
        click_at(view.editor, caret);
        open_menu(view);
        inject_named_key(NamedKey::ArrowDown);
        inject_keys("x");
        say(
            view,
            "a menu is open under size while the editor has the keys: Down put the ring on a row of the \
             menu and x went nowhere, the caret still stands behind size",
        );
        let seen = state(view);
        ensure!(
            seen == (SCAD.to_string(), caret, true, true),
            "with the menu open, after Down and a letter: {:?} at {:?}, editing {}, menu open {}",
            seen.0,
            seen.1,
            seen.2,
            seen.3
        );
        let ring = from_main(|| Focus::focused().is_ok());
        ensure!(ring, "Down put the ring on no row of the menu");
        checkpoint("the ring is on a row of the menu, the code is as it was")?;

        inject_named_key(NamedKey::Enter);
        wait_for_next_frame();
        let picked = from_main(move || view.picked);
        let seen = state(view);
        ensure!(picked.is_some(), "Enter picked no row of the menu");
        ensure!(
            seen == (SCAD.to_string(), caret, true, false),
            "after Enter on a row: {:?} at {:?}, editing {}, menu open {}",
            seen.0,
            seen.1,
            seen.2,
            seen.3
        );

        open_menu(view);
        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        let seen = state(view);
        ensure!(
            seen == (SCAD.to_string(), caret, true, false),
            "after Escape on the menu: {:?} at {:?}, editing {}, menu open {}",
            seen.0,
            seen.1,
            seen.2,
            seen.3
        );
        inject_keys("s");
        let seen = state(view);
        ensure!(
            seen.0 == SCAD.replacen("size", "sizes", 1) && seen.1 == CodePos::new(LINE, BYTE + 1),
            "a letter after the menu closed gave {:?} at {:?}",
            seen.0,
            seen.1
        );
        say(
            view,
            "Enter picked a row and closed the menu. Escape closed a second menu and the editor kept the \
             keys: the s typed after it stands behind size",
        );
        checkpoint("no menu, line 2 starts with sizes, the caret behind it")?;
        Ok(())
    }
}
