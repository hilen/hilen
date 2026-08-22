#![cfg(test)]

use serial_test::serial;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::deps::{hreads::set_current_thread_as_main, refs::main_lock::MainLock};

struct Data {
    a: i32,
}

impl Default for Data {
    fn default() -> Self {
        Self { a: 20 }
    }
}

static DATA: MainLock<Data> = MainLock::new();
static INIT_DATA: MainLock<Data> = MainLock::new();

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn test_main_lock() {
    set_current_thread_as_main();
    assert_eq!(DATA.a, 20);
    DATA.get_mut().a = 40;
    assert_eq!(DATA.a, 40);
    assert_eq!(DATA.set(Data { a: 77 }).a, 77);
    assert_eq!(DATA.a, 77);
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn test_get_or_init() {
    set_current_thread_as_main();

    let data = INIT_DATA.get_or_init(|| Data { a: 44 });

    assert_eq!(data.a, 44);

    assert_eq!(INIT_DATA.a, 44);
}

#[test]
#[serial]
#[should_panic(expected = "This operation can be called only from main thread")]
fn fail_main_lock() {
    std::thread::spawn(set_current_thread_as_main).join().unwrap();

    _ = DATA.a;
}

struct NonDefault {
    _a: i32,
}

static MANUAL_DATA: MainLock<NonDefault> = MainLock::new();

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn test_manual_init() {
    set_current_thread_as_main();

    assert!(!MANUAL_DATA.is_set());
    assert!(MANUAL_DATA.try_get().is_none());
    assert!(MANUAL_DATA.try_get_mut().is_none());

    MANUAL_DATA.set(NonDefault { _a: 55 });

    assert!(MANUAL_DATA.is_set());
    assert!(MANUAL_DATA.try_get().is_some());
    assert!(MANUAL_DATA.try_get_mut().is_some());
}
