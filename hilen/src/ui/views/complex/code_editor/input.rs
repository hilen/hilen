//! The touches, the keys, the clipboard and the right click menu of a
//! `CodeEditor`.

use plat::Platform;

use super::{edit::LinesChanged, view::CodeEditor};
use crate::{
    deps::refs::Weak,
    gm::{LossyConvert, flat::Point},
    system::Clipboard,
    ui::{ContextMenu, Focus, Input, MenuItem, UIEvents, UIManager, View, ViewFrame, ViewTouch},
    window::NamedKey,
};

/// Page Up and Page Down move by the lines on screen less this many, so
/// a line of the page before stays in view.
const PAGE_OVERLAP: usize = 2;

impl CodeEditor {
    pub(super) fn setup_touch(self: Weak<Self>) {
        self.enable_touch();
        self.touch().began.val(move |touch| self.pressed(touch.position));
        self.touch().moved.val(move |touch| self.dragged(touch.position));
        self.touch().all.val(move |touch| {
            if touch.is_ended() {
                self.released();
            }
        });
        self.touch().up_inside.val(self, move |touch| self.tapped(touch.position));
        self.touch().secondary.val(self, move |touch| self.show_menu(touch.position));
    }

    pub(super) fn editing_changed(mut self: Weak<Self>, selected: bool) {
        self.editing = selected;
        if selected {
            UIEvents::keyboard_key().val(self, move |key| self.on_key(key));
            UIEvents::keyboard_input().val(self, move |ch| self.on_char(ch));
            Focus::hold_keys(self.weak_view());
        } else {
            UIEvents::keyboard_input().unsubscribe(self);
            UIEvents::keyboard_key().unsubscribe(self);
            Focus::release_keys(self.weak_view());
            self.dragging = false;
        }
        // The caret shows only while the editor has the keys.
        self.table.reload_data();
    }

    /// A press with a mouse puts the cursor there, 2 quick ones take the
    /// word, 3 the line, Shift with a press makes the selection reach it.
    /// Where a drag scrolls, a touch screen, the cursor goes there on the
    /// tap instead, so a scroll does not move it.
    fn pressed(self: Weak<Self>, position: Point) {
        if UIManager::drag_scrolling() || self.on_find_bar(position) {
            return;
        }
        self.place(position);
    }

    fn tapped(self: Weak<Self>, position: Point) {
        if UIManager::drag_scrolling() && !self.on_find_bar(position) {
            self.place(position);
        }
    }

    /// A press on the find bar, between its fields and buttons, is no
    /// press on the text under it.
    fn on_find_bar(&self, position: Point) -> bool {
        position.y < self.table.y()
    }

    fn place(mut self: Weak<Self>, position: Point) {
        let at = self.pos_at(position);
        self.click_count = if self.clicks.tap_in_row(position) {
            self.click_count + 1
        } else {
            1
        };
        match self.click_count {
            1 => {
                let extend = Input::modifiers().shift_key();
                self.text.move_to(at, extend);
                self.dragging = true;
            }
            2 => self.text.select_word(at),
            _ => self.text.select_line(at.line),
        }
        self.cursor_changed();
    }

    fn dragged(mut self: Weak<Self>, position: Point) {
        if !self.dragging {
            return;
        }
        let at = self.pos_at(position);
        if at != self.text.cursor() {
            self.text.move_to(at, true);
            self.cursor_changed();
        }
    }

    fn released(mut self: Weak<Self>) {
        self.dragging = false;
    }

    fn change(mut self: Weak<Self>, changed: Option<LinesChanged>) {
        if let Some(changed) = changed {
            self.lines_changed(changed);
        }
    }

    fn on_char(mut self: Weak<Self>, ch: char) {
        if self.is_null() || !self.is_selected() {
            return;
        }
        if Input::command_held() {
            self.on_command(Input::shortcut_char(ch));
            return;
        }
        // Enter, Tab, Backspace, Delete and Escape come as named keys too,
        // they are handled there.
        if ch.is_control() {
            return;
        }
        let changed = self.text.type_char(ch);
        self.lines_changed(changed);
    }

    fn on_command(mut self: Weak<Self>, ch: char) {
        let modifiers = Input::modifiers();
        let shift = modifiers.shift_key();
        match ch.to_ascii_lowercase() {
            // Cmd H hides the app on a Mac, so replace is Cmd Alt F there.
            'f' if modifiers.alt_key() && Platform::MAC => self.find_from_keys(true),
            'h' if !Platform::MAC => self.find_from_keys(true),
            'f' => self.find_from_keys(false),
            'g' if shift => self.find_previous(),
            'g' => self.find_next(),
            'a' => self.select_all(),
            'c' => self.copy(),
            'x' => self.cut(),
            'v' => self.paste(),
            'z' if shift => self.redo(),
            'z' => self.undo(),
            'y' => self.redo(),
            _ => {}
        }
    }

    fn on_key(mut self: Weak<Self>, key: NamedKey) {
        if self.is_null() || !self.is_selected() {
            return;
        }
        let modifiers = Input::modifiers();
        let shift = modifiers.shift_key();
        let command = Input::command_held();
        let word = if Platform::MAC {
            modifiers.alt_key()
        } else {
            modifiers.control_key()
        };
        // Escape closes an open find bar first.
        if key == NamedKey::Escape && self.find.open {
            self.close_find();
            return;
        }
        let text = &mut self.text;

        match key {
            NamedKey::Enter => {
                let changed = text.new_line();
                self.change(Some(changed));
                return;
            }
            NamedKey::Tab => {
                let changed = if shift {
                    text.outdent()
                } else {
                    Some(text.indent())
                };
                self.change(changed);
                return;
            }
            NamedKey::Backspace => {
                let changed = text.delete_backward();
                self.change(changed);
                return;
            }
            NamedKey::Delete => {
                let changed = text.delete_forward();
                self.change(changed);
                return;
            }
            NamedKey::Escape => {
                if text.selection().is_some() {
                    let cursor = text.cursor();
                    text.move_to(cursor, false);
                } else {
                    UIManager::unselect_view();
                    return;
                }
            }
            NamedKey::ArrowLeft if command && Platform::MAC => text.move_home(shift),
            NamedKey::ArrowRight if command && Platform::MAC => text.move_end(shift),
            NamedKey::ArrowLeft if word => text.move_word_left(shift),
            NamedKey::ArrowRight if word => text.move_word_right(shift),
            NamedKey::ArrowLeft => text.move_left(shift),
            NamedKey::ArrowRight => text.move_right(shift),
            NamedKey::ArrowUp | NamedKey::Home if command => text.move_to_start(shift),
            NamedKey::ArrowDown | NamedKey::End if command => text.move_to_end(shift),
            NamedKey::ArrowUp => text.move_lines(-1, shift),
            NamedKey::ArrowDown => text.move_lines(1, shift),
            NamedKey::Home => text.move_home(shift),
            NamedKey::End => text.move_end(shift),
            NamedKey::PageUp => {
                let page = self.page();
                self.text.move_lines(-page, shift);
            }
            NamedKey::PageDown => {
                let page = self.page();
                self.text.move_lines(page, shift);
            }
            _ => return,
        }
        self.cursor_changed();
    }

    /// The lines a Page Up or Page Down moves by.
    fn page(&self) -> isize {
        let rows: usize = (self.height() / self.line_height()).floor().max(1.0).lossy_convert();
        isize::try_from(rows.saturating_sub(PAGE_OVERLAP).max(1)).unwrap_or(1)
    }

    pub fn copy(&self) {
        let selected = self.text.selected_text();
        if selected.is_empty() {
            return;
        }
        if let Err(err) = Clipboard::set_text(&selected) {
            log::error!("Failed to copy from a code editor: {err}");
        }
    }

    pub fn cut(mut self: Weak<Self>) {
        self.copy();
        let changed = self.text.delete_selection();
        self.change(changed);
    }

    /// Puts the text of the clipboard in place of the selection.
    #[cfg(not_wasm)]
    pub fn paste(mut self: Weak<Self>) {
        match Clipboard::get_text() {
            Ok(text) => self.insert(&text),
            Err(err) => log::warn!("Nothing to paste into a code editor: {err}"),
        }
    }

    /// A browser has no paste here, a page reads the clipboard only
    /// through a call that answers later.
    #[cfg(wasm)]
    pub fn paste(self: Weak<Self>) {
        log::debug!(
            "No paste into a code editor in a browser, {} lines",
            self.line_count()
        );
    }

    /// A right click or a long press. Outside the selection it first puts
    /// the cursor there, a long press on a touch screen takes the word.
    fn show_menu(mut self: Weak<Self>, position: Point) {
        if self.on_find_bar(position) {
            return;
        }
        let at = self.pos_at(position);
        let inside = self.text.selection().is_some_and(|(start, end)| start <= at && at <= end);
        if !inside {
            if UIManager::drag_scrolling() {
                self.text.select_word(at);
            } else {
                self.text.move_to(at, false);
            }
            self.cursor_changed();
        }

        let selected = self.text.selection().is_some();
        let item = move |title: &str, on: bool, action: fn(Weak<Self>)| {
            let item = MenuItem::new(title, move || {
                action(self);
                self.select();
            });
            if on { item } else { item.disabled() }
        };
        let items = vec![
            item("Undo", self.can_undo(), |mut editor| editor.undo()),
            item("Redo", self.can_redo(), |mut editor| editor.redo()),
            MenuItem::separator(),
            item("Cut", selected, CodeEditor::cut),
            item("Copy", selected, |editor| editor.copy()),
            item("Paste", !Platform::WASM, CodeEditor::paste),
            MenuItem::separator(),
            item("Select All", true, |mut editor| editor.select_all()),
        ];
        let frame = self.absolute_frame();
        ContextMenu::show(items, Point::new(frame.x() + position.x, frame.y() + position.y));
    }
}
