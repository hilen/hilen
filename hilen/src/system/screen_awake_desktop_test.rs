use anyhow::{Result, ensure};

use super::ScreenAwake;
use crate::{
    self as hilen,
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// On a desktop the guard must reach the system, not only the engine's own
/// state. On macOS the power management lists every hold with its reason,
/// so the test reads that list: the hold is there while a guard lives and
/// gone after the last one drops. Windows and Linux have no list a test can
/// read without extra rights, there the test pins that the request goes
/// through without an error.
#[view]
struct ScreenAwakeDesktop {
    #[init]
    title: Label,
    state: Label,
}

impl Setup for ScreenAwakeDesktop {
    fn setup(self: Weak<Self>) {
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("the display stays on while a guard lives");
        self.state.set_frame((20, 100, 560, 40));
        self.state.set_text("screen awake: off");
    }
}

impl ViewTest for ScreenAwakeDesktop {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(!system_holds()?, "no hold before the first guard");

        let guard = from_main(move || {
            view.state.set_text("screen awake: on");
            ScreenAwake::acquire()
        });
        ensure!(system_holds()?, "the system must list the hold of a live guard");
        checkpoint("screen awake: on, the system lists the hold")?;

        drop(guard);
        from_main(move || {
            view.state.set_text("screen awake: off");
        });
        ensure!(!system_holds()?, "the hold must be gone after the guard drops");
        checkpoint("screen awake: off, the hold is gone")?;
        Ok(())
    }
}

/// The system's own answer to whether this app holds the display awake.
fn system_holds() -> Result<bool> {
    // The drop of a guard lands on the main thread, wait for it.
    let applied = from_main(|| super::STATE.get_mut().applied);

    #[cfg(macos)]
    {
        use std::process::Command;

        use anyhow::Context;

        let output = Command::new("pmset").args(["-g", "assertions"]).output().context("pmset")?;
        let listed = String::from_utf8_lossy(&output.stdout).contains(super::REASON);
        ensure!(
            listed == applied,
            "the engine says {applied} and the system says {listed}"
        );
        Ok(listed)
    }
    #[cfg(not(macos))]
    Ok(applied)
}
