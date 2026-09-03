use anyhow::{Result, anyhow};
use hilen::{
    dispatch::{from_main, on_main},
    inspect::{
        AppCommand, InspectService,
        protocol::{UIRequest, UIResponse},
        weak_to_id,
    },
    refs::Weak,
    ui::{Button, ModifiersState, Setup, UIManager, View, ViewFrame, ViewTest, ViewTouch, view},
};

/// A tap injected over the inspect protocol can hold modifiers, so a Cmd
/// click drives multi selection like a real mouse, and a right tap fires
/// the secondary action like a real right click. The held modifiers
/// release right after the tap, like the keys request.
#[view]
struct InspectTapModifiers {
    plain:     u32,
    cmd:       u32,
    secondary: u32,

    #[init]
    button: Button,
}

impl Setup for InspectTapModifiers {
    fn setup(mut self: Weak<Self>) {
        self.button.set_text("Tap");
        self.button.set_frame((20, 20, 120, 44));
        self.button.on_tap(move || {
            if UIManager::command_held() {
                self.cmd += 1;
            } else {
                self.plain += 1;
            }
        });
        self.button.touch().secondary.sub(self, move || {
            self.secondary += 1;
        });
    }
}

/// The tree in the reply holds `Own` pointers, which must drop on the main
/// thread like the transports do.
fn send_tap(view_id: String, modifiers: ModifiersState, right: bool) -> Result<()> {
    let response = InspectService::process_command(
        UIRequest::Tap {
            view_id,
            modifiers,
            right,
        }
        .into(),
    );
    let result = match &response {
        AppCommand::UI(UIResponse::SendUI { .. }) => Ok(()),
        other => Err(anyhow!("Unexpected inspect response: {other:?}")),
    };
    on_main(move || drop(response));
    result
}

impl ViewTest for InspectTapModifiers {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let counts = move || from_main(move || (view.plain, view.cmd, view.secondary));
        let button_id = from_main(move || weak_to_id(view.button.weak_view()));

        send_tap(button_id.clone(), ModifiersState::empty(), false)?;
        assert_eq!(counts(), (1, 0, 0));

        send_tap(button_id.clone(), ModifiersState::SUPER, false)?;
        assert_eq!(counts(), (1, 1, 0));

        // The held Cmd released with its request.
        assert!(!from_main(UIManager::command_held));
        send_tap(button_id.clone(), ModifiersState::empty(), false)?;
        assert_eq!(counts(), (2, 1, 0));

        // A right tap fires the secondary action, not the tap.
        send_tap(button_id, ModifiersState::empty(), true)?;
        assert_eq!(counts(), (2, 1, 1));

        Ok(())
    }
}
