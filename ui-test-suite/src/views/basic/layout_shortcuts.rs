use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{KeyCombo, ModifiersState, Setup, TextField, UIManager, ViewData, ViewTest, view},
    ui_test::{inject_layout_key, inject_modifiers},
    window::KeyCode,
};

/// A key binding names a Latin key and fires on every keyboard layout. A
/// layout that types other letters, Russian, Greek or Hebrew, offers the
/// Latin letter of the physical key after its own. A Latin layout like
/// AZERTY keeps its own letters, only its digit row offers the digits.
#[view]
struct LayoutShortcuts {
    bold:     usize,
    fit:      usize,
    first:    usize,
    cyrillic: usize,

    #[init]
    field: TextField,
}

impl Setup for LayoutShortcuts {
    fn setup(mut self: Weak<Self>) {
        self.field.set_text("selected by a Cyrillic key");
        self.field.place().t(40).lr(20).h(40);

        let keymap = UIManager::keymap();
        keymap.add(self, KeyCombo::cmd('b'), move || self.bold += 1);
        keymap.add(self, 'Z', move || self.fit += 1);
        keymap.add(self, KeyCombo::cmd('1'), move || self.first += 1);
        keymap.add(self, 'ж', move || self.cyrillic += 1);
    }
}

impl ViewTest for LayoutShortcuts {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_modifiers(ModifiersState::CONTROL);
        // Russian, Greek and Hebrew on the physical B key.
        for typed in ['и', 'β', 'נ'] {
            inject_layout_key(typed, KeyCode::KeyB);
        }
        // AZERTY types b on the B key, nothing to fall back to.
        inject_layout_key('b', KeyCode::KeyB);
        from_main(move || assert_eq!(view.bold, 4));

        // AZERTY types & and Lithuanian ą on the 1 key.
        inject_layout_key('&', KeyCode::Digit1);
        inject_layout_key('ą', KeyCode::Digit1);
        from_main(move || assert_eq!(view.first, 2));

        // A plain binding fires with no modifier too, Shift made the typed
        // letter upper case and the Latin one follows it.
        inject_modifiers(ModifiersState::SHIFT);
        inject_layout_key('Я', KeyCode::KeyZ);
        from_main(move || assert_eq!(view.fit, 1));

        // A binding on the typed letter takes the press, the Latin one of
        // the same key stays silent.
        inject_modifiers(ModifiersState::empty());
        inject_layout_key('ж', KeyCode::Semicolon);
        from_main(move || assert_eq!(view.cyrillic, 1));

        // An iPhone edits in the system text field, its own Cmd and A.
        #[cfg(not(all(ios, not(tvos))))]
        {
            from_main(move || view.field.focus_with_keyboard());
            inject_modifiers(ModifiersState::CONTROL);
            inject_layout_key('ф', KeyCode::KeyA);
            inject_modifiers(ModifiersState::empty());
            from_main(move || assert_eq!(view.field.selected_text(), view.field.text()));
        }

        Ok(())
    }
}
