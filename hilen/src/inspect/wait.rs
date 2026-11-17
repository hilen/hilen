use std::sync::mpsc::channel;

use crate::{
    deps::hreads::{after, from_main},
    gm::LossyConvert,
};

/// The longest wait a request may ask for.
pub(super) const MAX_WAIT_MS: u32 = 60_000;

/// Blocks the inspector worker for `ms` on the engine's `after`, so the
/// main thread keeps drawing frames while the request waits.
pub(super) fn wait_ms(ms: u32) -> Result<(), String> {
    let (send, recv) = channel();
    from_main(move || {
        let delay: f32 = ms.lossy_convert();
        after(delay / 1000.0, move || {
            send.send(()).expect("Inspect wait receiver is gone");
        });
    });
    recv.recv().map_err(|error| format!("Wait failed: {error}"))
}
