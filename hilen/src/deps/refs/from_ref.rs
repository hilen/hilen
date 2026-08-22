use core::ptr::from_ref;

use crate::deps::refs::{Weak, ref_counter::RefCounter};

pub fn weak_from_ref<T: ?Sized>(rf: &T) -> Weak<T> {
    let ptr = from_ref::<T>(rf).cast_mut();
    assert!(!ptr.is_null(), "Failed to get ptr from ref");

    let address = ptr.cast::<u8>() as usize;

    let Some(stamp) = RefCounter::stamp_for_address(address) else {
        panic!("Trying to get weak pointer for object which is not managed by reference counter.")
    };

    Weak {
        ptr,
        stamp,
        type_name: std::any::type_name::<T>(),
    }
}

#[cfg(test)]
mod test {
    use std::ops::Deref;

    use serial_test::serial;

    use crate::deps::{
        hreads::set_current_thread_as_main,
        refs::{Own, weak_from_ref},
    };

    #[test]
    #[serial]
    #[should_panic(
        expected = "Trying to get weak pointer for object which is not managed by reference counter."
    )]
    fn test() {
        set_current_thread_as_main();
        let five = Own::new(5);
        let five = five.deref();

        let weak = weak_from_ref(five);

        assert_eq!(*weak.deref(), 5);

        let five = 5;
        let five = &five;

        let _ = weak_from_ref(five);
    }
}
