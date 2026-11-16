use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    ui::{Setup, ViewFrame, ViewSubviews, ViewTest, view},
};

/// Many frames of a phone, and no other event in it.
const WAIT: Duration = Duration::from_millis(500);

static FREED: AtomicBool = AtomicBool::new(false);

#[view]
struct Removed {}

impl Setup for Removed {
    fn setup(self: Weak<Self>) {}
}

impl Drop for Removed {
    fn drop(&mut self) {
        FREED.store(true, Ordering::Relaxed);
    }
}

/// A removed view is freed in the next frame with no other event. The
/// iOS simulator kept an edited field this way, and its keyboard stayed
/// up, until something else woke the loop. The check reads only an atomic,
/// a call on the main thread would wake the loop itself.
#[view]
struct RemovedViewIsFreed {
    removed: Weak<Removed>,
}

impl Setup for RemovedViewIsFreed {
    fn setup(mut self: Weak<Self>) {
        self.removed = self.add_view::<Removed>();
        self.removed.set_frame((20, 20, 40, 40));
    }
}

impl ViewTest for RemovedViewIsFreed {
    fn before_start() {
        FREED.store(false, Ordering::Relaxed);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            let mut removed = view.removed;
            removed.remove_from_superview();
        });
        sleep(WAIT);
        ensure!(
            FREED.load(Ordering::Relaxed),
            "a removed view was still alive {} ms later",
            WAIT.as_millis()
        );
        Ok(())
    }
}
