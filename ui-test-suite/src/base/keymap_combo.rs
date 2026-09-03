use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{KeyCombo, ModifiersState, NamedKey, Setup, UIManager, ViewTest, view},
    ui_test::{inject_key, inject_modifiers, inject_named_key},
};

#[view]
struct KeymapCombo {
    plain:     u32,
    cmd:       u32,
    cmd_shift: u32,
    cmd_enter: u32,
}

impl Setup for KeymapCombo {
    fn setup(mut self: Weak<Self>) {
        UIManager::keymap().add(self, 'b', move || {
            self.plain += 1;
        });

        UIManager::keymap().add(self, KeyCombo::cmd('b'), move || {
            self.cmd += 1;
        });

        UIManager::keymap().add(self, KeyCombo::cmd_shift('b'), move || {
            self.cmd_shift += 1;
        });

        UIManager::keymap().add(self, KeyCombo::cmd(NamedKey::Enter), move || {
            self.cmd_enter += 1;
        });
    }
}

impl ViewTest for KeymapCombo {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let counts = move || from_main(move || (view.plain, view.cmd, view.cmd_shift, view.cmd_enter));

        assert_eq!(counts(), (0, 0, 0, 0));

        inject_key('b');
        assert_eq!(counts(), (1, 0, 0, 0));

        inject_modifiers(ModifiersState::SUPER);
        inject_key('b');
        assert_eq!(counts(), (1, 1, 0, 0));

        // Ctrl counts as the command modifier too, for non Mac desktops.
        inject_modifiers(ModifiersState::CONTROL);
        inject_key('b');
        assert_eq!(counts(), (1, 2, 0, 0));

        inject_modifiers(ModifiersState::SUPER | ModifiersState::SHIFT);
        inject_key('b');
        assert_eq!(counts(), (1, 2, 1, 0));

        inject_modifiers(ModifiersState::SUPER);
        inject_named_key(NamedKey::Enter);
        assert_eq!(counts(), (1, 2, 1, 1));

        // A plain enter reaches no combo.
        inject_modifiers(ModifiersState::empty());
        inject_named_key(NamedKey::Enter);
        assert_eq!(counts(), (1, 2, 1, 1));

        // Shift alone changes the arriving character, not the binding, so
        // it does not block a plain binding.
        inject_modifiers(ModifiersState::SHIFT);
        inject_key('b');
        assert_eq!(counts(), (2, 2, 1, 1));

        inject_modifiers(ModifiersState::empty());
        inject_key('b');
        assert_eq!(counts(), (3, 2, 1, 1));

        Ok(())
    }
}
