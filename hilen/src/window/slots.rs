//! Which window the engine works on right now. The main window is slot 0, an
//! extra window takes a free slot from 1 on, see `AppWindow`. The state that
//! belongs to 1 window, like the hovered view or the damage of the last frame,
//! is kept per slot in a `WindowLock`, so the code that reads it does not
//! change with the number of windows.

use std::{
    ops::Deref,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

#[cfg(all(not_wasm, feature = "gpu"))]
use log::debug;

use crate::deps::{hreads::assert_main_thread, refs::main_lock::MainLock};

/// The main window and up to 7 extra windows at once.
pub(crate) const MAX_WINDOWS: usize = 8;

static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// A slot gets a new generation each time it is given out, so a value kept
/// for a closed window is made again for the next one in that slot.
static GENERATIONS: [AtomicU32; MAX_WINDOWS] = [const { AtomicU32::new(0) }; MAX_WINDOWS];

/// The slot of the window the engine works on. Its events, its layout and
/// its frame run with it current. Everything else runs with the main window.
pub(crate) fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub(crate) fn is_main() -> bool {
    current() == 0
}

/// Runs `action` with the window of `slot` current, then goes back to the
/// window that was current before.
pub(crate) fn within<R>(slot: usize, action: impl FnOnce() -> R) -> R {
    assert_main_thread();
    assert!(slot < MAX_WINDOWS, "No window slot {slot}");
    let before = CURRENT.swap(slot, Ordering::Relaxed);
    let result = action();
    CURRENT.store(before, Ordering::Relaxed);
    result
}

/// The window of `slot` closed. When it was current, the rest of its event
/// runs as the main window, so a draw for the main window that its action
/// started does not look for the window that is gone.
#[cfg(all(not_wasm, feature = "gpu"))]
pub(crate) fn leave(slot: usize) {
    assert_main_thread();
    if CURRENT.compare_exchange(slot, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        debug!("Window {slot} closed in its own event, the rest runs as the main window");
    }
}

pub(crate) fn generation(slot: usize) -> u32 {
    GENERATIONS[slot].load(Ordering::Relaxed)
}

/// A new window takes `slot`. Every value kept for the window that had it
/// before is made again on its next read.
#[cfg(all(not_wasm, feature = "gpu"))]
pub(crate) fn renew(slot: usize) {
    assert!(slot > 0, "The main window keeps its slot");
    GENERATIONS[slot].fetch_add(1, Ordering::Relaxed);
}

/// A `MainLock` per window slot. It derefs to the lock of the current
/// window, so every call of `MainLock` works on it as it is.
pub(crate) struct WindowLock<T> {
    locks:   [MainLock<T>; MAX_WINDOWS],
    /// The generation of its slot each lock was last used in.
    made_in: [AtomicU32; MAX_WINDOWS],
}

impl<T> Default for WindowLock<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> WindowLock<T> {
    pub(crate) const fn new() -> Self {
        Self {
            locks:   [const { MainLock::new() }; MAX_WINDOWS],
            made_in: [const { AtomicU32::new(0) }; MAX_WINDOWS],
        }
    }

    fn lock_of(&self, slot: usize) -> &MainLock<T> {
        assert_main_thread();
        let lock = &self.locks[slot];
        let now = generation(slot);
        if self.made_in[slot].swap(now, Ordering::Relaxed) != now {
            lock.clear();
        }
        lock
    }

    /// Takes the value out of `slot`, for a window that closes and has to
    /// free what it holds now, not at the next open of its slot.
    #[cfg(all(not_wasm, feature = "gpu"))]
    pub(crate) fn take_from(&self, slot: usize) -> Option<T> {
        self.lock_of(slot).take()
    }
}

impl<T> Deref for WindowLock<T> {
    type Target = MainLock<T>;

    fn deref(&self) -> &Self::Target {
        self.lock_of(current())
    }
}

/// An `AtomicU32` per window slot, for a value that is read off the main
/// thread too, like the UI scale. A read off the main thread gives the value
/// of the window that is current on the main thread.
pub(crate) struct WindowU32 {
    slots: [AtomicU32; MAX_WINDOWS],
}

impl WindowU32 {
    pub(crate) const fn new(value: u32) -> Self {
        Self {
            slots: [const { AtomicU32::new(0) }; MAX_WINDOWS],
        }
        .filled(value)
    }

    const fn filled(self, value: u32) -> Self {
        let mut slots = self.slots;
        let mut slot = 0;
        while slot < MAX_WINDOWS {
            slots[slot] = AtomicU32::new(value);
            slot += 1;
        }
        Self { slots }
    }

    pub(crate) fn load(&self) -> u32 {
        self.slots[current()].load(Ordering::Relaxed)
    }

    pub(crate) fn swap(&self, value: u32) -> u32 {
        self.slots[current()].swap(value, Ordering::Relaxed)
    }
}

/// The slots of the windows that are open. A build with no `gpu` feature
/// opens no window, its views live in the main slot.
#[cfg(not(feature = "gpu"))]
pub(crate) fn open_slots() -> Vec<usize> {
    vec![0]
}

#[cfg(all(test, feature = "gpu"))]
mod test {
    use serial_test::serial;

    use super::{WindowLock, current, leave, renew, within};
    use crate::deps::hreads::set_current_thread_as_main;

    static VALUE: WindowLock<i32> = WindowLock::new();

    #[test]
    #[serial]
    fn every_window_has_its_own_value() {
        set_current_thread_as_main();
        *VALUE.get_mut() = 1;
        within(3, || *VALUE.get_mut() = 3);

        assert_eq!(**VALUE, 1);
        assert_eq!(within(3, || **VALUE), 3);
        assert_eq!(current(), 0);
    }

    // A window that opens in the slot of a closed one must not find the hover
    // or the damage the closed one left there.
    #[test]
    #[serial]
    fn a_reused_slot_starts_fresh() {
        set_current_thread_as_main();
        within(4, || *VALUE.get_mut() = 44);
        renew(4);

        assert_eq!(within(4, || **VALUE), 0);
    }

    // A key in the screenshot window of banda closed it and then drew a card
    // of the main window, which panicked with "Window 1 is not up".
    #[test]
    #[serial]
    fn a_window_closed_in_its_own_event_hands_over_to_the_main_window() {
        set_current_thread_as_main();
        let after_close = within(2, || {
            leave(2);
            current()
        });
        let other_closed = within(3, || {
            leave(2);
            current()
        });

        assert_eq!(after_close, 0);
        assert_eq!(other_closed, 3);
        assert_eq!(current(), 0);
    }

    #[test]
    #[serial]
    fn a_closed_window_gives_its_value_back() {
        set_current_thread_as_main();
        within(5, || *VALUE.get_mut() = 55);

        assert_eq!(VALUE.take_from(5), Some(55));
        assert!(!within(5, || VALUE.is_set()));
    }
}
