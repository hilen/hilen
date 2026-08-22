use log::trace;

use crate::{
    deps::{hreads::on_main, refs::Weak},
    ui::{Spinner, ViewSubviews},
};

pub struct SpinnerLockGlobal {
    pub(crate) stopped: bool,
}

impl SpinnerLockGlobal {
    pub fn animated_stop(mut self) {
        self.stopped = true;
        Spinner::stop();
    }
    pub fn stop(self) {}
}

impl Drop for SpinnerLockGlobal {
    fn drop(&mut self) {
        trace!("SpinnerLockGlobal dropped");
        if !self.stopped {
            Spinner::instant_stop();
        }
    }
}

#[derive(Default)]
pub struct SpinnerLockOnView {
    pub spinner: Weak<Spinner>,
}

impl SpinnerLockOnView {
    pub fn is_active(&self) -> bool {
        self.spinner.is_ok()
    }
    pub fn stop(self) {}
}

impl Drop for SpinnerLockOnView {
    fn drop(&mut self) {
        if self.spinner.is_ok() {
            trace!("SpinnerLockOnView dropped");
            let mut spinner = self.spinner;
            on_main(move || {
                spinner.remove_from_superview();
            });
        }
    }
}
