use std::{
    collections::hash_map::Entry,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use rustc_hash::FxHashMap;

use crate::deps::refs::{
    __internal_deps::{RwLock, RwLockReadGuard, RwLockWriteGuard},
    Stamp,
    own::Addr,
};

static COUNTER: OnceLock<RefCounter> = OnceLock::new();

// Checked on every `Weak` deref, so the hot path of the whole frame loop.
// SipHash is overkill for keys that are heap addresses.
type Map = FxHashMap<Addr, Stamp>;

#[derive(Default)]
pub(crate) struct RefCounter {
    deallocators: RwLock<Map>,
}

impl RefCounter {
    fn counter() -> RwLockReadGuard<'static, Map> {
        COUNTER.get_or_init(RefCounter::default).deallocators.read()
    }

    fn counter_mut() -> RwLockWriteGuard<'static, Map> {
        COUNTER.get_or_init(RefCounter::default).deallocators.write()
    }

    pub(crate) fn stamp_for_address(addr: Addr) -> Option<Stamp> {
        Self::counter().get(&addr).copied()
    }

    pub(crate) fn add(
        addr: Addr,
        #[cfg(feature = "pointers_info")] location: &'static std::panic::Location,
    ) -> Stamp {
        let stamp = stamp();
        match Self::counter_mut().entry(addr) {
            Entry::Vacant(slot) => {
                slot.insert(stamp);
            }
            Entry::Occupied(slot) => {
                unreachable!(
                    "Adding deallocator of already existing address: {addr:#x} (existing stamp: {})",
                    slot.get()
                );
            }
        }

        #[cfg(feature = "pointers_info")]
        crate::deps::refs::pointers_info::PointerInfo::record_alloc(addr, stamp, location);

        stamp
    }

    pub(crate) fn remove(addr: Addr, #[cfg(feature = "pointers_info")] backtrace: std::backtrace::Backtrace) {
        Self::counter_mut().remove(&addr).expect("Removing non existing address");
        #[cfg(feature = "pointers_info")]
        crate::deps::refs::pointers_info::PointerInfo::record_dealloc(addr, stamp(), backtrace);
    }
}

fn stamp() -> Stamp {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
