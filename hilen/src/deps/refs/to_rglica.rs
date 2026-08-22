use core::ptr::from_ref;
use std::ptr::NonNull;

use crate::deps::refs::Rglica;

pub trait ToRglica<T: ?Sized> {
    fn to_rglica(&self) -> Rglica<T>;
}

impl<T: ?Sized> ToRglica<T> for Box<T> {
    fn to_rglica(&self) -> Rglica<T> {
        let ptr = NonNull::new((from_ref::<T>(self.as_ref())).cast_mut());
        debug_assert!(ptr.is_some(), "Failed to make Rglica from Box");
        Rglica {
            ptr: ptr.unwrap().into(),
        }
    }
}

impl<T: ?Sized> ToRglica<T> for &T {
    fn to_rglica(&self) -> Rglica<T> {
        Rglica::from_ref(self)
    }
}

impl<T: ?Sized> ToRglica<T> for &mut T {
    fn to_rglica(&self) -> Rglica<T> {
        Rglica::from_ref(self)
    }
}

#[cfg(test)]
mod test {
    use std::ops::Deref;

    use serial_test::serial;

    use crate::deps::{
        hreads::set_current_thread_as_main,
        refs::{Own, ToRglica},
    };

    #[test]
    #[serial]
    fn test() {
        set_current_thread_as_main();

        let five_ref = &5_i32;
        let five = five_ref.to_rglica();
        assert_eq!(*five.deref(), 5);

        let five_mut = &mut 5_i32;
        let five = five_mut.to_rglica();
        assert_eq!(*five.deref(), 5);

        let five_box = Box::new(5);
        let five = five_box.to_rglica();
        assert_eq!(*five.deref(), 5);

        let five_own = Own::new(5);
        let five_weak = five_own.weak();
        let five = unsafe { five_weak.to_rglica() };
        assert_eq!(*five.deref(), 5);
    }
}
