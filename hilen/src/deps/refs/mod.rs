#![allow(incomplete_features)]
#![allow(internal_features)]

mod as_any;
mod erased;
mod from_ref;
mod into_own;
mod own;
#[cfg(feature = "pointers_info")]
mod pointers_info;
mod raw_pointer;
mod ref_counter;
mod rglica;
mod serde;
mod to_rglica;
mod weak;

pub use as_any::*;
pub use erased::*;
pub use from_ref::*;
pub use own::*;
pub use raw_pointer::*;
pub use rglica::*;
pub use to_rglica::*;
pub use weak::*;

pub mod main_lock;
pub mod manage;
mod tests;
pub mod vec;

pub mod hreads {
    pub use crate::deps::hreads::set_current_thread_as_main;
}

pub mod __internal_deps {
    pub use log::warn;
    /// The browser main thread must never park, a contended parking
    /// lock there raises an Atomics.wait error and kills the page. The
    /// managed storage critical sections are single map operations, so
    /// on wasm they spin instead.
    #[cfg(not(target_arch = "wasm32"))]
    pub use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
    #[cfg(target_arch = "wasm32")]
    pub use spin::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
}

#[cfg(feature = "stats")]
pub mod stats;
