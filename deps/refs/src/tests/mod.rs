#![cfg(test)]

use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    thread::spawn,
};

use hreads::set_current_thread_as_main;
use pretty_assertions::assert_eq;
use serial_test::serial;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::{AsAny, Own, Weak, ref_counter::RefCounter};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn weak_misc() {
    set_current_thread_as_main();
    let five = Own::new(5);
    let ten = Own::new(10);

    assert_ne!(five, ten);

    let mut weak = five.weak();
    let another_weak = weak;

    assert!(!weak.is_null());
    assert_eq!(weak.deref(), another_weak.deref());

    let null = Weak::<i32>::default();

    assert!(null.is_null());
    assert!(!null.is_ok());
    assert_eq!(null.get(), None);

    let five_ref = weak.get_mut().unwrap();

    assert_eq!(five_ref, &5);

    *five_ref = 10;

    assert_eq!(weak.deref(), &10);

    assert!(!weak.is_null());
    assert!(weak.is_ok());
    assert_eq!(weak.get(), Some(10).as_ref());

    set_current_thread_as_main();
    drop(five);

    assert!(weak.is_null());
    assert!(!weak.is_ok());
    assert_eq!(weak.get(), None);
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn leak_weak() {
    set_current_thread_as_main();
    let leaked = unsafe { Weak::leak(5) };
    dbg!(leaked.deref());
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
#[should_panic(expected = "Invalid address. In could be a closure or empty type.")]
fn leak_weak_closure() {
    let _leaked = unsafe { Weak::leak(|| {}) };
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn addr() {
    set_current_thread_as_main();

    let own = Own::new(5);
    let weak = own.weak();
    assert_eq!(own.addr(), weak.addr());
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
#[should_panic(expected = "Dereferencing never initialized weak pointer: i32")]
fn null_weak_panic() {
    let default = Weak::<i32>::default();
    assert!(!default.is_ok());
    let _ = default.deref();
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
#[should_panic(expected = "Dereferencing already freed weak pointer: i32")]
fn freed_unsized_weak_panic() {
    set_current_thread_as_main();
    let own = Own::new(5);
    let weak: Weak<dyn Any> = own.weak();
    drop(own);

    assert_eq!(weak.type_name, "i32");
    assert!(!weak.is_ok());
    let _ = weak.deref();
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn const_weak_default() {
    const WEAK: Weak<bool> = Weak::const_default();
    set_current_thread_as_main();
    assert!(WEAK.is_null());
}

#[serial]
#[should_panic(expected = "Dereferencing never initialized weak pointer: u32")]
#[wasm_bindgen_test(unsupported = test)]
fn deref_null() {
    set_current_thread_as_main();
    let null = Weak::<u32>::default();
    assert!(null.is_null());
    assert!(!null.is_ok());
    let _ = null.deref();
}

#[serial]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value")]
#[wasm_bindgen_test(unsupported = test)]
fn deref_async() {
    set_current_thread_as_main();
    let num = Own::new(5);
    let mut weak = num.weak();
    spawn(move || {
        assert_eq!(weak.deref(), &5);
        assert_eq!(weak.deref_mut(), &5);
    })
    .join()
    .unwrap();
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn default_weak() {
    trait Trait {
        fn _a(&self);
    }

    let weak = Weak::<i32>::default();
    assert!(weak.is_null());

    let weak = Weak::<dyn Trait>::default();
    assert!(weak.is_null());
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn downcast_weak() {
    trait Tr: AsAny {}
    struct St {
        a: i32,
    }

    impl Tr for St {}
    impl AsAny for St {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn into_any_box(self: Box<Self>) -> Box<dyn Any> {
            self
        }
    }

    set_current_thread_as_main();

    let own: Own<dyn Tr> = Own::new(St { a: 50 });
    let downcasted: Weak<St> = own.downcast_weak().unwrap();

    assert_eq!(downcasted.a, 50);
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn downcast_own() {
    trait Tr: AsAny {}
    struct St {
        a: i32,
    }

    impl Tr for St {}
    impl AsAny for St {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn into_any_box(self: Box<Self>) -> Box<dyn Any> {
            self
        }
    }

    set_current_thread_as_main();

    let own: Own<dyn Tr> = Own::new(St { a: 100 });
    let downcasted: Own<St> = own.downcast::<St>();

    assert_eq!(downcasted.a, 100);
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn weak_map_key() {
    struct NonHash {
        _a: u8,
    }

    set_current_thread_as_main();

    let own = Own::new(NonHash { _a: 0 });
    let weak = own.weak();

    let mut map: HashMap<Weak<NonHash>, u32> = HashMap::new();
    map.entry(weak).or_insert(5);
    assert_eq!(map.get(&weak).unwrap(), &5);
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn was_initialized() {
    set_current_thread_as_main();

    let a = Weak::<i32>::default();
    let b = Own::new(5);
    let b = b.weak();

    assert!(!a.was_initialized());
    assert!(b.was_initialized());
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn raw_and_dump() {
    set_current_thread_as_main();

    let a = Own::new(5);
    let a = a.weak();
    let erased = a.erase();

    let raw = a.raw();

    let from_raw: Weak<i32> = unsafe { Weak::from_raw(raw) };
    let from_raw_unsized: Weak<dyn Any> = unsafe { Weak::from_raw(raw) };

    assert_eq!(a.raw(), from_raw.raw());
    assert_eq!(a.raw(), from_raw_unsized.raw());
    assert_eq!(a.raw(), erased.raw());
}

#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn stale_weak_after_address_reuse() {
    set_current_thread_as_main();

    let own = Own::new(10_u64);
    let mut stale = own.weak();

    // A weak made before this address was reused carries the stamp of the
    // older object. No allocator can be told to reuse an address, so the older
    // stamp is set by hand. Looping until a real reuse happened made this test
    // panic with "inconclusive" on a macOS CI runner.
    stale.stamp -= 1;

    assert!(stale.is_null());
    assert!(!stale.is_ok());
    assert_eq!(stale.get(), None);
    assert_eq!(*own, 10);
}

/// Dropping an `Own` must unregister its address, otherwise the next object
/// that lands there cannot register and `RefCounter::add` panics.
#[serial]
#[wasm_bindgen_test(unsupported = test)]
fn address_is_free_after_drop() {
    set_current_thread_as_main();

    let own = Own::new(5_u64);
    let weak = own.weak();
    let addr = own.addr();
    drop(own);

    assert!(weak.is_null());
    assert!(RefCounter::stamp_for_address(addr).is_none());
}
