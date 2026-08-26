use anyhow::{Result, anyhow};
use hilen::{
    dispatch::{from_main, on_main},
    inspect::{
        AppCommand, InspectService,
        protocol::{Key, UIRequest},
    },
    refs::Weak,
    ui::{KeyCombo, ModifiersState, NamedKey, Setup, UIManager, ViewData, ViewTest, view},
};

/// Keys injected over the inspect protocol must reach the keymap and the
/// focused text field like a real keyboard, and the modifiers they hold
/// must release right after, so a Cmd from one request never leaks into
/// the next input.
#[view]
struct InspectKeys {
    plain:     u32,
    cmd:       u32,
    cmd_enter: u32,
    #[cfg(desktop)]
    #[init]
    field:     hilen::ui::TextField,
}

impl Setup for InspectKeys {
    fn setup(mut self: Weak<Self>) {
        UIManager::keymap().add(self, 'p', move || {
            self.plain += 1;
        });

        UIManager::keymap().add(self, KeyCombo::cmd('p'), move || {
            self.cmd += 1;
        });

        UIManager::keymap().add(self, KeyCombo::cmd(NamedKey::Enter), move || {
            self.cmd_enter += 1;
        });

        #[cfg(desktop)]
        self.field.place().tl(20).size(300, 40);
    }
}

/// The tree in the reply holds `Own` pointers, which must drop on the main
/// thread like the transports do.
fn send(keys: Vec<Key>, modifiers: ModifiersState) -> Result<()> {
    let response = InspectService::process_command(UIRequest::Keys { keys, modifiers }.into());
    let result = match &response {
        AppCommand::UI(_) => Ok(()),
        other => Err(anyhow!("Unexpected inspect response: {other:?}")),
    };
    on_main(move || drop(response));
    result
}

impl ViewTest for InspectKeys {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let counts = move || from_main(move || (view.plain, view.cmd, view.cmd_enter));

        send(vec![Key::Char('p')], ModifiersState::empty())?;
        assert_eq!(counts(), (1, 0, 0));

        send(vec![Key::Char('p')], ModifiersState::SUPER)?;
        assert_eq!(counts(), (1, 1, 0));

        // The held Cmd released with its request.
        assert!(!from_main(UIManager::command_held));
        send(vec![Key::Char('p')], ModifiersState::empty())?;
        assert_eq!(counts(), (2, 1, 0));

        send(vec![Key::Named(NamedKey::Enter)], ModifiersState::SUPER)?;
        assert_eq!(counts(), (2, 1, 1));
        assert!(!from_main(UIManager::command_held));

        send(vec![Key::Named(NamedKey::Enter)], ModifiersState::empty())?;
        assert_eq!(counts(), (2, 1, 1));

        // A phone types through its screen keyboard, so a field only
        // receives injected chars on desktop.
        #[cfg(desktop)]
        {
            from_main(move || view.field.focus());
            send(
                vec![Key::Char('a'), Key::Char('b'), Key::Char('c')],
                ModifiersState::empty(),
            )?;
            assert_eq!(from_main(move || view.field.text().to_string()), "abc");

            // One request mixes text and named keys in order. Backspace
            // edits like the real key, and Enter ends the session, so the
            // last char goes nowhere.
            send(
                vec![
                    Key::Named(NamedKey::Backspace),
                    Key::Named(NamedKey::Enter),
                    Key::Char('z'),
                ],
                ModifiersState::empty(),
            )?;
            assert_eq!(from_main(move || view.field.text().to_string()), "ab");
        }

        Ok(())
    }
}
