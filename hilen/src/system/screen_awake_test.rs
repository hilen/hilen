use anyhow::{Result, ensure};

#[cfg(not(ios))]
use super::STATE;
use super::ScreenAwake;
use crate::{
    self as hilen,
    dispatch::from_main,
    refs::Weak,
    system::{
        AppActivity,
        app_activity::{self, ActivityChange},
    },
    ui::{Button, Label, Setup, ViewData, ViewTest, view},
    ui_test::{checkpoint, inject_touches},
};

#[view]
struct ScreenAwakeLifecycle {
    guard:  Option<ScreenAwake>,
    #[init]
    toggle: Button,
    status: Label,
}

impl Setup for ScreenAwakeLifecycle {
    fn setup(mut self: Weak<Self>) {
        self.toggle.set_text("Start / Stop").place().tl(20).size(240, 60);
        self.status.set_text("Stopped").place().l(20).t(100).size(500, 60);
        self.toggle.on_tap(move || {
            if self.guard.is_some() {
                self.guard = None;
                self.status.set_text("Stopped");
            } else {
                self.guard = Some(ScreenAwake::acquire());
                self.status.set_text("Running");
            }
        });
        AppActivity::changed().val(self, move |active| {
            if !active {
                self.guard = None;
                self.status.set_text("Stopped");
            }
        });
    }
}

impl ViewTest for ScreenAwakeLifecycle {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(restore_activity);
        let restore = RestoreActivity;
        inject_touches("100 50 b\n100 50 e");
        ensure!(from_main(screen_is_awake), "start must request an awake screen");
        checkpoint("Running: screen awake requested")?;

        // An independent owner must survive the first owner's release.
        let second = from_main(ScreenAwake::acquire);
        inject_touches("100 50 b\n100 50 e");
        ensure!(
            from_main(screen_is_awake),
            "another owner still needs the display"
        );
        drop(second);
        ensure!(!from_main(screen_is_awake), "last release must restore timeout");
        checkpoint("Stopped: normal screen timeout")?;

        for change in [
            ActivityChange::Focused(false),
            ActivityChange::Resumed(false),
            ActivityChange::Visible(false),
        ] {
            inject_touches("100 50 b\n100 50 e");
            from_main(move || app_activity::update(change));
            ensure!(!from_main(screen_is_awake), "inactive app must release display");
            ensure!(
                from_main(move || view.guard.is_none()),
                "inactive app must stop its work"
            );
            from_main(restore_activity);
            ensure!(!from_main(screen_is_awake), "return must not restart the request");
            ensure!(
                from_main(move || view.status.text() == "Stopped"),
                "return must stay stopped"
            );
        }
        checkpoint("Returned: stopped until Start is pressed")?;
        drop(restore);
        Ok(())
    }
}

fn restore_activity() {
    app_activity::update(ActivityChange::Focused(true));
    app_activity::update(ActivityChange::Visible(true));
    app_activity::update(ActivityChange::Resumed(true));
}

struct RestoreActivity;

impl Drop for RestoreActivity {
    fn drop(&mut self) {
        from_main(restore_activity);
    }
}

// On iOS assert UIKit's actual idle timer, not only the engine's request.
fn screen_is_awake() -> bool {
    #[cfg(ios)]
    {
        use objc2_foundation::MainThreadMarker;
        use objc2_ui_kit::UIApplication;

        UIApplication::sharedApplication(MainThreadMarker::new().expect("main thread")).isIdleTimerDisabled()
    }
    #[cfg(not(ios))]
    {
        STATE.get_mut().applied
    }
}
