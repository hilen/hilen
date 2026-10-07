//! The keys the owner of a text field takes for itself.

use super::TextField;
use crate::{
    deps::refs::weak_from_ref,
    ui::{UIManager, ViewSubviews},
    window::NamedKey,
};

impl TextField {
    /// The owner of the field takes these keys for itself. While the field
    /// is edited, a press of one of them fires `key_taken` and nothing
    /// else: the field does not move its caret, add a line, submit, end
    /// the editing or select the next field, and no key binding of the
    /// screen runs. A held modifier changes nothing. Every other key
    /// works as before.
    ///
    /// A list of completions over the field takes the arrows, Tab, Enter
    /// and Escape while it is open, and calls `release_keys` when it
    /// closes. A new call replaces the keys of the call before.
    ///
    /// On an iPhone the system text field has the keys. Only Return
    /// reaches the engine from there, so only `NamedKey::Enter` can be
    /// taken.
    pub fn take_keys(&self, keys: impl IntoIterator<Item = NamedKey>) -> &Self {
        weak_from_ref(self).taken_keys = keys.into_iter().collect();
        self
    }

    /// Gives the keys of `take_keys` back to the field.
    pub fn release_keys(&self) -> &Self {
        weak_from_ref(self).taken_keys.clear();
        self
    }

    /// The owner takes a press of `key` now.
    pub(super) fn takes_key(&self, key: NamedKey) -> bool {
        self.is_editing && self.taken_keys.contains(&key)
    }

    /// Hands `key` to the owner of the edited field when it takes that
    /// key, and says whether it did. Nothing else sees the press then.
    pub(crate) fn offer_key(key: NamedKey) -> bool {
        let selected = UIManager::selected_view();
        if selected.is_null() {
            return false;
        }
        let Some(field) = selected.downcast_view::<Self>() else {
            return false;
        };
        if !field.takes_key(key) {
            return false;
        }
        field.key_taken.trigger(key);
        true
    }
}
