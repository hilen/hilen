use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Alert, Color, Container, Label, ModalView, NamedKey, Setup, UIManager, ViewData, ViewTest, WHITE,
        view,
    },
    ui_test::{checkpoint, inject_key, inject_named_key},
};

/// A screen that binds Escape as its back key, the way a screen swapped in
/// with `UIManager::set_view` does, and counts how often it was left. The
/// root view binds the g key, like a shortcut of the whole app. An alert
/// opens over the screen. Proves one Escape closes the alert and does not
/// leave the screen under it, the g key still answers over the alert, and
/// Escape leaves the screen again once the alert is gone.
#[view]
struct KeyBindingUnderModal {
    left:   u32,
    global: u32,

    #[init]
    screen: Container,
    title:  Label,
    counts: Label,
}

impl KeyBindingUnderModal {
    fn show_counts(self: Weak<Self>) {
        let (left, global) = (self.left, self.global);
        self.counts.set_text(format!("screen left: {left}, g key: {global}"));
    }

    fn counts(self: Weak<Self>) -> (u32, u32) {
        wait_for_next_frame();
        from_main(move || (self.left, self.global))
    }
}

impl Setup for KeyBindingUnderModal {
    fn setup(mut self: Weak<Self>) {
        self.screen.set_color(Color::rgb(0.85, 0.92, 1.0));
        self.screen.place().back();

        self.title.set_text("a screen, Escape leaves it").set_text_size(26);
        self.title.place().lrt(20).h(60);

        self.counts.set_text_size(26).set_text_color(WHITE);
        self.counts.set_color(Color::rgb(0.1, 0.2, 0.4));
        self.counts.place().lrb(20).h(60);
        self.show_counts();

        UIManager::keymap().add(self.screen, NamedKey::Escape, move || {
            self.left += 1;
            self.show_counts();
        });
        // The root view outlives the test, its binding stays in the keymap.
        UIManager::keymap().add(UIManager::root_view(), 'g', move || {
            if self.is_null() {
                return;
            }
            self.global += 1;
            self.show_counts();
        });
    }
}

impl ViewTest for KeyBindingUnderModal {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let alert = from_main(|| Alert::prepare_modally_with_input("An alert over the screen".to_string()));
        wait_for_next_frame();
        checkpoint("an alert is open over the screen")?;

        inject_key('g');
        ensure!(
            view.counts() == (0, 1),
            "the g key of the root view under an alert: {:?}",
            view.counts()
        );

        inject_named_key(NamedKey::Escape);
        ensure!(
            view.counts() == (0, 1),
            "Escape under an alert left the screen: {:?}",
            view.counts()
        );
        ensure!(
            from_main(move || alert.is_null()),
            "Escape did not close the alert"
        );
        checkpoint("Escape closed the alert, the screen was not left")?;

        inject_named_key(NamedKey::Escape);
        ensure!(
            view.counts() == (1, 1),
            "Escape with no alert open: {:?}",
            view.counts()
        );
        checkpoint("the second Escape left the screen once")?;

        Ok(())
    }
}
