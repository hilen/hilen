//! The key that seals the `SessionStore` file. It has two halves. One is built
//! into the binary by the build script, masked, from `HILEN_SESSION_KEY`. The
//! other is the id of this machine, so a file copied to another computer does
//! not open even with the binary next to it.

use std::{
    hint::black_box,
    sync::atomic::{AtomicBool, Ordering},
};

use sha2::{Digest, Sha256};

use crate::encrypt::EncryptionKey;

include!(concat!(env!("OUT_DIR"), "/session_key.rs"));

const CONTEXT: &[u8] = b"hilen session store v1";

/// The warning about the development key was written since the start. An
/// app asks for the key on every request to its backend, a line each time
/// fills the log.
static WARNED: AtomicBool = AtomicBool::new(false);

/// True only for the first call with this flag.
fn first_time(flag: &AtomicBool) -> bool {
    !flag.swap(true, Ordering::Relaxed)
}

pub(crate) fn session_key() -> EncryptionKey {
    if IS_DEV && first_time(&WARNED) {
        log::warn!("SessionStore uses the public development key, the build had no HILEN_SESSION_KEY");
    }

    let mut hasher = Sha256::new();
    hasher.update(CONTEXT);
    hasher.update(built_in());
    hasher.update(machine_id());
    hasher.finalize().into()
}

/// Undoes the mask of the build script. Without `black_box` the optimizer
/// folds the three constants back into the plain key inside the binary.
fn built_in() -> Vec<u8> {
    let masked = black_box(MASKED);
    let mask = black_box(MASK);
    let shift = black_box(SHIFT);

    masked
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ mask[(index + shift) % mask.len()])
        .collect()
}

/// Empty where the platform has no such id. Mobile and browser storage is
/// already closed to other apps, there the built in half stands alone.
#[cfg(desktop)]
fn machine_id() -> String {
    machine_uid::get().unwrap_or_else(|error| {
        log::warn!("no machine id for the session key: {error}");
        String::new()
    })
}

#[cfg(not(desktop))]
fn machine_id() -> String {
    String::new()
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicBool;

    use super::{IS_DEV, MASKED, built_in, first_time, session_key};

    #[test]
    fn warns_once() {
        let flag = AtomicBool::new(false);
        assert!(first_time(&flag));
        assert!(!first_time(&flag));
        assert!(!first_time(&flag));
    }

    #[test]
    fn mask_comes_off() {
        let key = built_in();
        assert_ne!(key.as_slice(), MASKED.as_slice());
        // Only a build with no key in its env carries the development one.
        assert_eq!(IS_DEV, key.starts_with(b"hilen development session key"));
    }

    #[test]
    fn key_is_stable() {
        assert_eq!(session_key(), session_key());
    }
}
