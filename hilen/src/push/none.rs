//! Every system but iOS, which has no push in the engine yet.

use anyhow::anyhow;

use crate::push::Done;

pub(super) fn request(done: Done) {
    done(Err(anyhow!(
        "push is only on iOS for now, see docs/push.md in hilen"
    )));
}
