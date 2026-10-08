use anyhow::{Result, anyhow, ensure};
use hilen::{
    dispatch::{from_main, on_main, wait_for_next_frame},
    gm::color::RED,
    inspect::{AppCommand, InspectService, InspectorCommand},
    refs::Weak,
    ui::{Container, Label, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::checkpoint,
};

/// A view that was removed is freed with the next turn of the loop, also
/// when that turn draws no frame, in a paused app or behind another
/// window. It stayed alive there, so a timer of a removed page still
/// found its page and went on asking a backend.
#[view]
struct RemovedWhilePaused {
    #[init]
    status: Label,
}

impl Setup for RemovedWhilePaused {
    fn setup(self: Weak<Self>) {
        self.status.set_text("the red view is added").place().t(40).lr(20).h(40);
    }
}

fn send(command: InspectorCommand, wanted: fn(&AppCommand) -> bool) -> Result<()> {
    let response = InspectService::process_command(command);
    let result = if wanted(&response) {
        Ok(())
    } else {
        Err(anyhow!("Unexpected inspect response: {response:?}"))
    };
    // An answer can hold `Own` pointers, they drop on the main thread.
    on_main(move || drop(response));
    result
}

impl ViewTest for RemovedWhilePaused {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let mut removed = from_main(move || {
            let child = view.add_view::<Container>();
            child.set_color(RED);
            child.place().t(120).l(20).size(200, 120);
            child
        });
        wait_for_next_frame();
        checkpoint("the red view is there")?;

        send(InspectorCommand::Pause, |response| {
            matches!(response, AppCommand::Paused { .. })
        })?;
        from_main(move || removed.remove_from_superview());
        // The next turn of the paused loop, it draws no frame.
        let alive = from_main(move || removed.is_ok());
        send(InspectorCommand::Resume, |response| {
            matches!(response, AppCommand::Ok)
        })?;
        ensure!(!alive, "A view removed in a paused app is still alive");

        from_main(move || {
            view.status.set_text("the red view is removed and freed");
        });
        wait_for_next_frame();
        checkpoint("the red view is gone")?;
        Ok(())
    }
}
