//! The key that seals the `SessionStore` file. It has two halves. One is built
//! into the binary from `HILEN_SESSION_KEY`, hidden by `obfstr`. The
//! other is the id of this machine, so a file copied to another computer does
//! not open even with the binary next to it.

use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use crate::encrypt::EncryptionKey;

/// The build had no `HILEN_SESSION_KEY` and took the public development key.
const IS_DEV: bool = matches!(env!("HILEN_SESSION_KEY_IS_DEV").as_bytes(), b"true");

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
    hasher.update(obfstr::obfbytes!(env!("HILEN_SESSION_KEY_BUILT").as_bytes()));
    hasher.update(machine_id());
    hasher.finalize().into()
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

    use super::{IS_DEV, first_time, session_key};

    #[test]
    fn warns_once() {
        let flag = AtomicBool::new(false);
        assert!(first_time(&flag));
        assert!(!first_time(&flag));
        assert!(!first_time(&flag));
    }

    #[test]
    fn key_is_stable() {
        assert_eq!(session_key(), session_key());
    }

    /// The test binary is built like the app, with the key through `obfstr`.
    /// The plain key must not be anywhere in it. The text searched for is
    /// kept reversed here, or this test would put it into the binary itself.
    #[test]
    fn the_plain_key_is_not_in_the_binary() {
        if !IS_DEV {
            panic!("this test knows only the development key, run it without HILEN_SESSION_KEY");
        }
        let key: Vec<u8> = "ti htiw spihs reven esaeler a ,yek noisses tnempoleved nelih"
            .bytes()
            .rev()
            .collect();
        let binary =
            std::fs::read(std::env::current_exe().expect("the test binary")).expect("read the test binary");
        assert!(
            !binary.windows(key.len()).any(|window| window == key.as_slice()),
            "the plain session key is in the binary"
        );
    }
}
