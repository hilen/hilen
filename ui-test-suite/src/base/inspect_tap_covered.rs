use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, on_main},
    inspect::{
        AppCommand, InspectService,
        protocol::{UIRequest, UIResponse},
        weak_to_id,
    },
    refs::Weak,
    ui::{Button, Color, Label, ModifiersState, Setup, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// What the app answered to a tap request.
#[derive(Debug, PartialEq)]
enum Reply {
    Tapped,
    /// Tapped, with the covering warning.
    Warned,
    Refused(String),
}

/// A blue target button with a red button over its center, and a green
/// button nothing covers. Each counts its taps, shown in the text under
/// them. A tap sent over the inspect protocol to the blue button is
/// refused and names the red one, and no button is pressed. The same tap
/// with `force` is sent and comes back with the warning. The green button
/// takes a plain tap. A tap on a covered view once went through with only
/// a warning and pressed a reset button.
#[view]
struct InspectTapCovered {
    target_taps: u32,
    cover_taps:  u32,
    free_taps:   u32,

    #[init]
    target: Button,
    cover:  Button,
    free:   Button,
    counts: Label,
}

impl InspectTapCovered {
    fn show_counts(self: Weak<Self>) {
        self.counts.set_text(format!(
            "blue: {}, red: {}, green: {}",
            self.target_taps, self.cover_taps, self.free_taps
        ));
    }

    fn counts(self: Weak<Self>) -> (u32, u32, u32) {
        from_main(move || (self.target_taps, self.cover_taps, self.free_taps))
    }
}

impl Setup for InspectTapCovered {
    fn setup(mut self: Weak<Self>) {
        self.target.set_text("target").set_text_size(24);
        self.target.set_color(Color::rgb(0.2, 0.4, 1.0));
        self.target.set_frame((40, 60, 300, 120));
        self.target.on_tap(move || {
            self.target_taps += 1;
            self.show_counts();
        });

        self.cover.set_text("cover").set_text_size(24);
        self.cover.set_color(Color::rgb(0.9, 0.2, 0.2));
        self.cover.set_frame((140, 90, 300, 60));
        self.cover.on_tap(move || {
            self.cover_taps += 1;
            self.show_counts();
        });

        self.free.set_text("free").set_text_size(24);
        self.free.set_color(Color::rgb(0.2, 0.7, 0.3));
        self.free.set_frame((40, 240, 300, 80));
        self.free.on_tap(move || {
            self.free_taps += 1;
            self.show_counts();
        });

        self.counts.set_text_size(24);
        self.counts.set_frame((40, 380, 520, 50));
        self.show_counts();
    }
}

/// The tree in the reply holds `Own` pointers, which must drop on the main
/// thread like the transports do.
fn send_tap(view_id: String, force: bool) -> Reply {
    let response = InspectService::process_command(
        UIRequest::Tap {
            view_id,
            modifiers: ModifiersState::empty(),
            right: false,
            force,
        }
        .into(),
    );
    let reply = match &response {
        AppCommand::UI(UIResponse::SendUI { note: None, .. }) => Reply::Tapped,
        AppCommand::UI(UIResponse::SendUI { note: Some(_), .. }) => Reply::Warned,
        AppCommand::Error(error) => Reply::Refused(error.clone()),
        other => Reply::Refused(format!("unexpected reply: {other:?}")),
    };
    on_main(move || drop(response));
    reply
}

impl ViewTest for InspectTapCovered {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (target, free) = from_main(move || {
            (
                weak_to_id(view.target.weak_view()),
                weak_to_id(view.free.weak_view()),
            )
        });
        checkpoint("the red button covers the center of the blue one")?;

        let Reply::Refused(error) = send_tap(target.clone(), false) else {
            anyhow::bail!("a tap on the covered button was not refused");
        };
        ensure!(
            error.contains("InspectTapCovered.cover") && error.contains("--force"),
            "the refusal does not name the cover and the flag: {error}"
        );
        ensure!(
            view.counts() == (0, 0, 0),
            "a refused tap pressed a button: {:?}",
            view.counts()
        );
        checkpoint("the tap on the blue button was refused, all counts are 0")?;

        ensure!(
            send_tap(target, true) == Reply::Warned,
            "a forced tap came back with no warning"
        );
        let (target_taps, cover_taps, free_taps) = view.counts();
        ensure!(
            target_taps + cover_taps == 1 && free_taps == 0,
            "a forced tap sent no touch: {:?}",
            view.counts()
        );
        checkpoint("the forced tap was sent")?;

        ensure!(
            send_tap(free, false) == Reply::Tapped,
            "a tap on a free button was not plain"
        );
        ensure!(
            view.counts().2 == 1,
            "the free button was not pressed: {:?}",
            view.counts()
        );
        checkpoint("the green button took a plain tap")?;

        Ok(())
    }
}
