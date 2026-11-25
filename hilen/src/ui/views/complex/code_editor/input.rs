//! The touches, the keys, the clipboard and the right click menu of a
//! `CodeEditor`.

use plat::Platform;

use super::{edit::LinesChanged, pairs::Typed, pointer::CodePress, view::CodeEditor};
use crate::{
    deps::{
        hreads::after,
        refs::{Weak, main_lock::MainLock},
    },
    gm::{LossyConvert, flat::Point},
    system::Clipboard,
    ui::{
        ContextMenu, Focus, Input, MenuItem, NamedKey, TouchStack, UIEvents, UIManager, View, ViewFrame,
        ViewTouch,
    },
};

/// Page Up and Page Down move by the lines on screen less this many, so
/// a line of the page before stays in view.
const PAGE_OVERLAP: usize = 2;

/// The whole line the last Cmd C or Cmd X with no selection took. A
/// paste of that same text goes above the line of the cursor, the way
/// Zed has it.
static LINE_CLIP: MainLock<String> = MainLock::new();

/// What Ctrl K cut last on a Mac, Ctrl Y puts it back.
static KILLED: MainLock<String> = MainLock::new();

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
        self.setup_pointer();
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
            self.hide_completions();
            // A note goes with the typing, and the typing ended.
            self.hide_note();
            self.keys_may_have_left();
        }
        // The caret shows only while the editor has the keys.
        self.table.reload_data();
    }

    /// The editor or a field of its find bar lost the keys. The view that
    /// took them is selected only after the one that lost them heard of
    /// it, so the check runs a moment later: keys that went into the find
    /// bar or came back to the editor fire nothing.
    pub(super) fn keys_may_have_left(self: Weak<Self>) {
        after(0.0, move || {
            if self.is_ok() && !self.editing && !self.find_bar.has_keys() {
                self.editing_ended.trigger(());
            }
        });
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
        self.hide_completions();
        let at = self.pos_at(position);
        self.click_count = if self.clicks.tap_in_row(position) {
            self.click_count + 1
        } else {
            1
        };
        let command = Input::command_held();
        match self.click_count {
            1 => {
                let extend = Input::modifiers().shift();
                self.text.move_to(at, extend);
                // A press with Cmd is a jump of the app, not the start of
                // a selection.
                self.dragging = !command;
            }
            2 => self.text.select_word(at),
            _ => self.text.select_line(at.line),
        }
        self.cursor_changed();
        if self.click_count == 1 {
            self.pressed.trigger(CodePress { pos: at, command });
        }
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

    /// The editor has the keys and no other touch layer lies over it. A
    /// menu or a modal over the editor takes the keys, also the ones of
    /// its own right click menu.
    fn takes_keys(self: Weak<Self>) -> bool {
        !self.is_null() && self.is_selected() && TouchStack::key_reaches(self.raw())
    }

    fn on_char(mut self: Weak<Self>, ch: char) {
        if !self.takes_keys() {
            return;
        }
        // Ctrl is no command key on a Mac, the text views of the system
        // move and delete with it.
        let modifiers = Input::modifiers();
        if Platform::MAC && modifiers.ctrl() && !modifiers.meta() {
            self.on_control(Input::shortcut_char(ch));
            return;
        }
        if Input::command_held() {
            self.on_command(Input::shortcut_char(ch));
            return;
        }
        // Enter, Tab, Backspace, Delete and Escape come as named keys too,
        // they are handled there.
        if ch.is_control() || self.read_only {
            return;
        }
        let rules = self.brackets.clone();
        let scopes = self.scope_runs(self.text.cursor().line).to_vec();
        match self.text.type_paired(ch, &rules, &scopes) {
            Typed::Changed(changed) => self.lines_changed(changed),
            Typed::Moved => self.cursor_changed(),
        }
    }

    fn on_command(mut self: Weak<Self>, ch: char) {
        let modifiers = Input::modifiers();
        let shift = modifiers.shift();
        match ch.to_ascii_lowercase() {
            // Cmd H hides the app on a Mac, so replace is Cmd Alt F there.
            'f' if modifiers.alt() && Platform::MAC => self.find_from_keys(true),
            'h' if !Platform::MAC => self.find_from_keys(true),
            'f' => self.find_from_keys(false),
            'g' if shift => self.find_previous(),
            'g' => self.find_next(),
            'a' => self.select_all(),
            'c' => self.copy(),
            'x' => self.cut(),
            'v' => self.paste(),
            'l' => {
                self.text.select_lines();
                self.cursor_changed();
            }
            'z' | 'y' | 'k' | '/' if self.read_only => {}
            'z' if shift => self.redo(),
            'z' => self.undo(),
            'y' => self.redo(),
            'k' if shift => {
                let changed = self.text.delete_lines();
                self.change(Some(changed));
            }
            '/' => {
                let comments = self.comments.clone();
                let changed = self.text.toggle_comment(&comments);
                self.change(changed);
            }
            _ => {}
        }
    }

    /// Ctrl with a letter on a Mac: the keys of the text views of the
    /// system that the Zed keymap has, and Ctrl J joins lines. Any other
    /// letter puts in nothing.
    fn on_control(mut self: Weak<Self>, ch: char) {
        let shift = Input::modifiers().shift();
        let ch = ch.to_ascii_lowercase();
        let page = self.page();
        let text = &mut self.text;
        match ch {
            'p' => text.move_lines(-1, shift),
            'n' => text.move_lines(1, shift),
            'b' => text.move_left(shift),
            'f' => text.move_right(shift),
            'a' => text.move_home(shift),
            'e' => text.move_end(shift),
            'v' => text.move_lines(page, shift),
            _ => {
                if !self.read_only {
                    let changed = self.control_edit(ch);
                    self.change(changed);
                }
                return;
            }
        }
        self.cursor_changed();
    }

    /// The Ctrl letters of a Mac that change the text.
    fn control_edit(mut self: Weak<Self>, ch: char) -> Option<LinesChanged> {
        let comments = self.comments.clone();
        let text = &mut self.text;
        match ch {
            'h' => text.delete_pair().or_else(|| text.delete_backward()),
            'd' => text.delete_forward(),
            'w' => text.delete_word_backward(),
            't' => text.transpose(),
            'j' => text.join_lines(&comments),
            'k' => text.kill_to_line_end().map(|(killed, changed)| {
                KILLED.set(killed);
                changed
            }),
            'y' => KILLED
                .try_get()
                .filter(|killed| !killed.is_empty())
                .cloned()
                .map(|killed| text.insert(&killed)),
            _ => None,
        }
    }

    fn on_key(mut self: Weak<Self>, key: NamedKey) {
        if !self.takes_keys() {
            return;
        }
        if self.completion_key(key) {
            return;
        }
        // Escape closes what floats at the code first: the list above,
        // then the note, and only then it drops a selection.
        if key == NamedKey::Escape && self.note.is_ok() {
            self.hide_note();
            return;
        }
        let modifiers = Input::modifiers();
        let shift = modifiers.shift();
        let command = Input::command_held();
        let word = if Platform::MAC {
            modifiers.alt()
        } else {
            modifiers.ctrl()
        };
        // Escape closes an open find bar first.
        if key == NamedKey::Escape && self.find.open {
            self.close_find();
            return;
        }
        let moves_lines =
            matches!(key, NamedKey::ArrowUp | NamedKey::ArrowDown) && modifiers.alt() && !command;
        let edits_text = moves_lines
            || matches!(
                key,
                NamedKey::Enter | NamedKey::Tab | NamedKey::Backspace | NamedKey::Delete
            );
        if edits_text {
            if !self.read_only {
                self.edit_key(key, moves_lines);
            }
            return;
        }
        let text = &mut self.text;

        match key {
            NamedKey::Escape => {
                text.end_snippet();
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

    /// Enter, Tab, Backspace, Delete and Alt Up or Down, the named keys
    /// that change the text.
    fn edit_key(mut self: Weak<Self>, key: NamedKey, moves_lines: bool) {
        let modifiers = Input::modifiers();
        let shift = modifiers.shift();
        let command = Input::command_held();
        let word = if Platform::MAC {
            modifiers.alt()
        } else {
            modifiers.ctrl()
        };
        let rules = self.brackets.clone();
        let scopes = self.scope_runs(self.text.cursor().line).to_vec();
        let text = &mut self.text;
        let changed = match key {
            NamedKey::Enter if command => Some(if shift {
                text.new_line_above()
            } else {
                text.new_line_below()
            }),
            NamedKey::Enter => Some(text.new_line_paired(&rules, &scopes)),
            NamedKey::Tab if (!shift && text.next_stop()) || (shift && text.previous_stop()) => {
                self.cursor_changed();
                return;
            }
            NamedKey::Tab if shift => text.outdent(),
            NamedKey::Tab => Some(text.indent()),
            NamedKey::Backspace if command && Platform::MAC => text.delete_to_line_start(),
            NamedKey::Backspace if word => text.delete_word_backward(),
            NamedKey::Backspace => text.delete_pair().or_else(|| text.delete_backward()),
            NamedKey::Delete if command && Platform::MAC => text.delete_to_line_end(),
            NamedKey::Delete if word => text.delete_word_forward(),
            NamedKey::Delete => text.delete_forward(),
            NamedKey::ArrowUp | NamedKey::ArrowDown if moves_lines => {
                let down = key == NamedKey::ArrowDown;
                if shift {
                    Some(text.duplicate_lines(!down))
                } else {
                    text.move_block(down)
                }
            }
            _ => None,
        };
        self.change(changed);
    }

    /// The lines a Page Up or Page Down moves by.
    fn page(&self) -> isize {
        let rows: usize = (self.height() / self.line_height()).floor().max(1.0).lossy_convert();
        isize::try_from(rows.saturating_sub(PAGE_OVERLAP).max(1)).unwrap_or(1)
    }

    /// Copies the selection, or the whole line of the cursor with its
    /// break when nothing is selected.
    pub fn copy(&self) {
        let selected = self.text.selected_text();
        if selected.is_empty() {
            let line = self.text.line_with_break();
            put_on_clipboard(&line);
            LINE_CLIP.set(line);
        } else {
            put_on_clipboard(&selected);
            LINE_CLIP.clear();
        }
    }

    /// Cuts the selection, or the whole line of the cursor with its break
    /// when nothing is selected. A read only editor copies.
    pub fn cut(mut self: Weak<Self>) {
        if self.read_only {
            self.copy();
            return;
        }
        if self.text.selection().is_none() {
            let (line, changed) = self.text.cut_line();
            put_on_clipboard(&line);
            LINE_CLIP.set(line);
            self.lines_changed(changed);
            return;
        }
        self.copy();
        let changed = self.text.delete_selection();
        self.change(changed);
    }

    /// Puts the text of the clipboard in place of the selection.
    #[cfg(not_wasm)]
    pub fn paste(mut self: Weak<Self>) {
        if self.read_only {
            return;
        }
        match Clipboard::get_text() {
            Ok(text) if self.text.selection().is_none() && LINE_CLIP.try_get() == Some(&text) => {
                let changed = self.text.paste_line(&text);
                self.lines_changed(changed);
            }
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
        let writes = !self.read_only;
        let item = move |title: &str, on: bool, action: fn(Weak<Self>)| {
            let item = MenuItem::new(title, move || {
                action(self);
                self.select();
            });
            if on { item } else { item.disabled() }
        };
        let items = vec![
            item("Undo", writes && self.can_undo(), |mut editor| editor.undo()),
            item("Redo", writes && self.can_redo(), |mut editor| editor.redo()),
            MenuItem::separator(),
            item("Cut", writes && selected, CodeEditor::cut),
            item("Copy", selected, |editor| editor.copy()),
            item("Paste", writes && !Platform::WASM, CodeEditor::paste),
            MenuItem::separator(),
            item("Select All", true, |mut editor| editor.select_all()),
        ];
        let frame = self.absolute_frame();
        ContextMenu::show(items, Point::new(frame.x() + position.x, frame.y() + position.y));
    }
}

fn put_on_clipboard(text: &str) {
    if let Err(err) = Clipboard::set_text(text) {
        log::error!("Failed to copy from a code editor: {err}");
    }
}
