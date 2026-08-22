use std::sync::atomic::{AtomicU64, Ordering};

use log::error;

static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
static MAIN_THREAD_ID: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static THREAD_ID: u64 = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn current_thread_id() -> u64 {
    THREAD_ID.with(|id| *id)
}

pub fn assert_main_thread() {
    let is_main = is_main_thread();

    if !is_main {
        error!("This operation can be called only from main thread");
    }

    assert!(is_main, "This operation can be called only from main thread");
}

#[inline]
pub fn is_main_thread() -> bool {
    current_thread_id() == supposed_main_id()
}

pub fn set_current_thread_as_main() {
    MAIN_THREAD_ID.store(current_thread_id(), Ordering::Relaxed);
}

#[inline]
fn supposed_main_id() -> u64 {
    let id = MAIN_THREAD_ID.load(Ordering::Relaxed);

    assert_ne!(
        id, 0,
        "Main thread is not set. Call set_current_thread_as_main() first."
    );

    id
}

#[cfg(test)]
mod test {
    use std::sync::atomic::Ordering;

    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::deps::hreads::{
        is_main_thread,
        main_thread::{MAIN_THREAD_ID, supposed_main_id},
        set_current_thread_as_main,
    };

    #[test]
    #[serial]
    #[should_panic(expected = "Main thread is not set")]
    fn unset_main_panics() {
        MAIN_THREAD_ID.store(0, Ordering::Relaxed);
        is_main_thread();
    }

    #[serial]
    #[wasm_bindgen_test(unsupported = test)]
    fn test() {
        MAIN_THREAD_ID.store(5, Ordering::Relaxed);
        assert_eq!(supposed_main_id(), 5);

        set_current_thread_as_main();
        assert!(is_main_thread());
    }

    #[test]
    #[serial]
    fn other_thread_is_not_main() {
        set_current_thread_as_main();
        assert!(is_main_thread());

        std::thread::spawn(|| {
            assert!(!is_main_thread());
        })
        .join()
        .unwrap();

        assert!(is_main_thread());
    }

    #[test]
    #[serial]
    fn thread_ids_are_unique() {
        let current = crate::deps::hreads::current_thread_id();
        assert_ne!(current, 0);

        let other = std::thread::spawn(crate::deps::hreads::current_thread_id).join().unwrap();

        assert_ne!(other, 0);
        assert_ne!(current, other);
    }
}
