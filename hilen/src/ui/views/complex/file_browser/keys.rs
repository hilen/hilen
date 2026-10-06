use super::browser::{FileBrowser, FileBrowserMode};
use crate::{
    deps::refs::Weak,
    gm::{Clock, flat::Point},
    ui::{Focus, Input, NamedKey, TouchStack, UIManager, View, ViewData},
};

/// Letters typed this close together make one name to jump to.
const TYPE_AHEAD_MS: f64 = 1000.0;

/// The keyboard of the browser. It has the keys after a press on its
/// list, and gives them back on a press anywhere else.
impl FileBrowser {
    /// The arrow keys and Enter go to the list from now on, the key
    /// focus ring stays out of them.
    pub fn take_keys(mut self: Weak<Self>) {
        self.keys_held = true;
        Focus::hold_keys(self.weak_view());
    }

    fn drop_keys(mut self: Weak<Self>) {
        self.keys_held = false;
        Focus::release_keys(self.weak_view());
    }

    pub(super) fn pointer_pressed(self: Weak<Self>, at: Point) {
        let on_list = !self.is_hidden_in_tree()
            && self.list.rows_frame().contains(at)
            && TouchStack::key_reaches(self.raw());

        if on_list {
            self.take_keys();
        } else if self.keys_held {
            self.drop_keys();
        }
    }

    fn keys_active(&self) -> bool {
        self.keys_held
            && !UIManager::text_editing()
            && !self.is_hidden_in_tree()
            && TouchStack::key_reaches(self.weak().raw())
    }

    pub(super) fn on_key(self: Weak<Self>, key: NamedKey) {
        if !self.keys_active() {
            return;
        }

        let command = Input::command_held();
        let per_row = isize::try_from(self.list.columns_per_row()).unwrap_or(1);
        let grid = self.list.is_grid();

        match key {
            NamedKey::ArrowUp if command => self.go_up(),
            NamedKey::ArrowDown if command => self.open_cursor(),
            NamedKey::ArrowDown => self.step(per_row),
            NamedKey::ArrowUp => self.step(-per_row),
            NamedKey::ArrowRight if grid => self.step(1),
            NamedKey::ArrowLeft if grid => self.step(-1),
            // The list has no use for them, the ring moves on to the
            // sidebar or the toolbar with the same press.
            NamedKey::ArrowLeft | NamedKey::ArrowRight => {
                self.drop_keys();
                Focus::on_key(key);
            }
            NamedKey::Home => self.step(isize::MIN / 2),
            NamedKey::End => self.step(isize::MAX / 2),
            NamedKey::Enter => self.open_cursor(),
            NamedKey::Backspace => self.go_up(),
            _ => (),
        }
    }

    pub(super) fn on_char(mut self: Weak<Self>, character: char) {
        if !self.keys_active() || Input::command_held() || character.is_control() {
            return;
        }

        let now = Clock::now_ms();
        if now - self.typed.1 > TYPE_AHEAD_MS {
            self.typed.0.clear();
        }
        self.typed.0.extend(character.to_lowercase());
        self.typed.1 = now;

        let this = self;
        let found = self.shown.iter().enumerate().position(|(index, entry)| {
            entry.name.to_lowercase().starts_with(&this.typed.0) && this.can_pick(index)
        });

        if let Some(index) = found {
            self.selection.only(index);
            self.selection_moved();
            self.list.reveal(index);
        }
    }

    pub(super) fn select_all(mut self: Weak<Self>) {
        if !self.keys_active() || !self.multi() {
            return;
        }
        let count = self.shown.len();
        self.selection.select_all(count);
        self.selection_moved();
    }

    fn step(mut self: Weak<Self>, delta: isize) {
        let extend = Input::modifiers().shift_key() && self.multi();
        let count = self.shown.len();
        let Some(index) = self.selection.step(delta, count, extend) else {
            return;
        };
        self.selection_moved();
        self.list.reveal(index);
    }

    /// Enter: the row under the cursor opens. With no row picked a pick
    /// mode takes it as Choose, the open folder is the choice then.
    fn open_cursor(self: Weak<Self>) {
        match self.selection.cursor().filter(|index| self.selection.contains(*index)) {
            Some(index) => self.open_entry(index),
            None if self.mode != FileBrowserMode::Browse => self.choose(),
            None => (),
        }
    }
}
