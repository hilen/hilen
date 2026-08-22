use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{NamedKey, Setup, UIManager, ViewTest, view},
    ui_test::{inject_key, inject_named_key},
};

#[view]
struct KeymapNamedKey {
    left:  u32,
    right: u32,
    chars: u32,
}

impl Setup for KeymapNamedKey {
    fn setup(mut self: Weak<Self>) {
        UIManager::keymap().add(self, NamedKey::ArrowLeft, move || {
            self.left += 1;
        });

        UIManager::keymap().add(self, NamedKey::ArrowRight, move || {
            self.right += 1;
        });

        UIManager::keymap().add(self, 'g', move || {
            self.chars += 1;
        });
    }
}

impl ViewTest for KeymapNamedKey {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let counts = move || from_main(move || (view.left, view.right, view.chars));

        assert_eq!(counts(), (0, 0, 0));

        inject_named_key(NamedKey::ArrowLeft);
        assert_eq!(counts(), (1, 0, 0));

        inject_named_key(NamedKey::ArrowRight);
        inject_named_key(NamedKey::ArrowRight);
        assert_eq!(counts(), (1, 2, 0));

        inject_named_key(NamedKey::Enter);
        assert_eq!(counts(), (1, 2, 0));

        inject_key('g');
        assert_eq!(counts(), (1, 2, 1));

        inject_named_key(NamedKey::ArrowLeft);
        assert_eq!(counts(), (2, 2, 1));

        Ok(())
    }
}
