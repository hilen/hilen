#![allow(clippy::mut_from_ref)]

use std::{cell::UnsafeCell, ops::Deref};

use crate::deps::hreads::assert_main_thread;

#[derive(Default)]
pub struct MainLock<T> {
    val: UnsafeCell<Option<T>>,
}

unsafe impl<T> Send for MainLock<T> {}
unsafe impl<T> Sync for MainLock<T> {}

impl<T> MainLock<T> {
    pub const fn new() -> Self {
        Self {
            val: UnsafeCell::new(None),
        }
    }

    fn get_internal(&self) -> &mut Option<T> {
        assert_main_thread();
        unsafe { self.val.get().as_mut().unwrap() }
    }

    pub fn get_or_init(&self, init: impl Fn() -> T) -> &mut T {
        let rf = self.get_internal();

        if rf.is_none() {
            *rf = Some(init());
        }

        rf.as_mut().unwrap()
    }

    pub fn set(&self, value: T) -> &mut T {
        let rf = self.get_internal();
        *rf = Some(value);
        rf.as_mut().unwrap()
    }

    pub fn is_set(&self) -> bool {
        self.get_internal().is_some()
    }

    pub fn try_get(&self) -> Option<&T> {
        self.get_internal().as_ref()
    }

    pub fn try_get_mut(&self) -> Option<&mut T> {
        self.get_internal().as_mut()
    }

    /// # Safety
    ///
    /// Caller must ensure that this call is performed on main thread
    /// and that walue was already initialized
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_unchecked(&self) -> &mut T {
        unsafe { self.val.get().as_mut().unwrap().as_mut().unwrap() }
    }
}

impl<T: Default> MainLock<T> {
    pub fn get_mut(&self) -> &mut T {
        self.get_or_init(|| T::default())
    }
}

impl<T: Default + 'static> Deref for MainLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_mut()
    }
}
