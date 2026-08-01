use std::{
    any::type_name,
    fmt::{Debug, Formatter},
    marker::Unsize,
    ops::{CoerceUnsized, Deref, DerefMut},
    ptr::from_ref,
};

use hreads::is_main_thread;

use crate::{AsAny, PTR_SIZE, RawPointer, Weak, ref_counter::RefCounter};

pub(crate) type Stamp = u64;
pub(crate) type Addr = usize;

pub struct Own<T: ?Sized> {
    bx:        Box<T>,
    stamp:     Stamp,
    type_name: &'static str,
}

unsafe impl<T: ?Sized> Send for Own<T> {}
unsafe impl<T: ?Sized> Sync for Own<T> {}

impl<T: Sized + 'static> Own<T> {
    #[cfg_attr(feature = "pointers_info", track_caller)]
    pub fn new(val: T) -> Self {
        let type_name = std::any::type_name::<T>();

        // #[cfg(feature = "stats")]
        // crate::stats::adjust_stat(type_name, 1);

        let val = Box::new(val);
        let address = from_ref::<T>(&val).cast::<u8>() as usize;

        assert_ne!(
            address, 1,
            "Invalid address. In could be a closure or empty type."
        );

        #[cfg(feature = "pointers_info")]
        let stamp = RefCounter::add(address, std::panic::Location::caller());

        #[cfg(not(feature = "pointers_info"))]
        let stamp = RefCounter::add(address);

        Self {
            bx: val,
            stamp,
            type_name,
        }
    }
}

impl<T: ?Sized + AsAny> Own<T> {
    pub fn downcast<U: 'static>(self) -> Own<U> {
        let (bx, stamp) = unsafe {
            let bx = std::ptr::read(std::ptr::addr_of!(self.bx));
            let stamp = self.stamp;
            std::mem::forget(self);
            (bx, stamp)
        };

        let any_box = bx.into_any_box();
        let bx = any_box.downcast::<U>().unwrap_or_else(|_| {
            panic!(
                "Failed to downcast box from {} to {}",
                type_name::<T>(),
                type_name::<U>()
            );
        });

        Own {
            bx,
            stamp,
            type_name: std::any::type_name::<U>(),
        }
    }

    pub fn downcast_weak<U: 'static>(&self) -> Option<Weak<U>> {
        self.weak().downcast()
    }
}

impl<T: ?Sized> Own<T> {
    #[cfg(feature = "checks")]
    fn check() {
        assert!(
            hreads::is_main_thread(),
            "Unsafe Own pointer deref: {}. Thread is not Main. Thread id: {}",
            std::any::type_name::<T>(),
            hreads::current_thread_id()
        );
    }
}

impl<T: ?Sized> Own<T> {
    pub(crate) fn addr(&self) -> usize {
        from_ref::<T>(self.bx.as_ref()).cast::<u8>() as usize
    }
}

impl<T: ?Sized> Drop for Own<T> {
    #[track_caller]
    fn drop(&mut self) {
        if !is_main_thread() {
            log::error!("Dropping Own<{}> on non main thread", type_name::<T>());
            panic!("Dropping Own<{}> on non main thread", type_name::<T>());
        }

        // #[cfg(feature = "stats")]
        // crate::stats::adjust_stat(self.type_name, -1);
        #[cfg(feature = "pointers_info")]
        RefCounter::remove(self.addr(), std::backtrace::Backtrace::capture());
        #[cfg(not(feature = "pointers_info"))]
        RefCounter::remove(self.addr());
    }
}

impl<T: ?Sized> Deref for Own<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.bx.deref()
    }
}

impl<T: ?Sized> DerefMut for Own<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "checks")]
        Self::check();
        self.bx.deref_mut()
    }
}

impl<T: ?Sized> Own<T> {
    pub fn weak(&self) -> Weak<T> {
        Weak {
            ptr:       self.ptr(),
            stamp:     self.stamp,
            type_name: self.type_name,
        }
    }

    pub fn sized(&self) -> bool {
        let ptr_size = size_of_val(&self.ptr());

        if ptr_size == PTR_SIZE {
            true
        } else if ptr_size == PTR_SIZE * 2 {
            false
        } else {
            unreachable!("Invalid ptr size: {ptr_size}")
        }
    }

    pub fn ptr(&self) -> *mut T {
        (&raw const *self.bx).cast_mut()
    }

    pub fn raw(&self) -> RawPointer {
        RawPointer::new(self.addr(), self.stamp, self.type_name)
    }
}

impl<T: Default + Sized + 'static> Default for Own<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized + Debug> Debug for Own<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Own<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ?Sized + PartialEq> PartialEq<T> for Own<T> {
    fn eq(&self, other: &T) -> bool {
        self.deref().eq(other)
    }
}

impl<T, U> CoerceUnsized<Own<U>> for Own<T>
where
    T: Unsize<U> + ?Sized,
    U: ?Sized,
{
}

#[cfg(test)]
mod tests {
    use std::{
        ops::{Deref, DerefMut},
        sync::atomic::{AtomicU64, Ordering},
    };

    use hreads::set_current_thread_as_main;
    use serial_test::serial;

    use crate::Own;

    #[test]
    #[serial]
    fn deref() {
        set_current_thread_as_main();
        let num = Own::new(5);
        assert_eq!(num.deref(), &5);
        assert_eq!(num.weak().deref(), &5);
    }

    #[test]
    #[serial]
    #[should_panic(expected = "Invalid address. In could be a closure or empty type.")]
    fn own_from_closure() {
        set_current_thread_as_main();
        let _ = Own::new(|| {});
    }

    #[test]
    #[serial]
    fn deref_mut() {
        set_current_thread_as_main();
        let mut num = Own::new(5);
        *num = 10;
        assert_eq!(num.deref(), &10);
        assert_eq!(num.weak().deref_mut(), &10);
    }

    #[test]
    #[serial]
    #[should_panic(expected = "Dereferencing already freed weak pointer: i32")]
    fn deref_freed() {
        set_current_thread_as_main();
        let num = Own::new(5);
        let weak = num.weak();
        drop(num);
        let _ = weak.deref();
    }

    static VAL: AtomicU64 = AtomicU64::new(0);

    #[test]
    #[serial]
    fn check_drop() {
        struct ToDrop {
            _a: bool,
        }

        impl Drop for ToDrop {
            fn drop(&mut self) {
                VAL.store(20, Ordering::Relaxed);
            }
        }

        set_current_thread_as_main();

        assert_eq!(VAL.load(Ordering::Relaxed), 0);

        let num = Own::new(ToDrop { _a: false });
        let weak = num.weak();
        assert!(!weak.is_null());
        drop(num);
        assert!(weak.is_null());

        assert_eq!(VAL.load(Ordering::Relaxed), 20);
    }

    #[test]
    #[serial]
    fn misc() {
        set_current_thread_as_main();

        let five = Own::new(5);
        let ten = Own::new(10);
        let another_five = Own::new(5);
        let five_int = 5;

        assert_eq!(five, another_five);
        assert_ne!(five, ten);
        assert_eq!(five, 5);
        assert_ne!(five, 10);
        assert_eq!("5", &format!("{five:?}"));
        assert_eq!(five, five_int);
    }
}
