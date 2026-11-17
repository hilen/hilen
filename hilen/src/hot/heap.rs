//! The heap of a hot build, see `docs/hot-reload.md`.
//!
//! A stopped build is never unloaded, so nothing ever runs the drop of what
//! its statics and its caches still hold. With the heap of the process that
//! memory would stay for good, about 10 MB per build. So every build takes
//! its Rust memory from a heap of its own, a malloc zone, and the loader
//! throws the whole zone away some time after the stop.
//!
//! The price: memory of a stopped build that anything still reads is a
//! crash, where it was a silent leftover before.

use std::{
    alloc::{GlobalAlloc, Layout},
    ffi::{c_uint, c_void},
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering},
};

/// `malloc_zone_t`, only ever behind a pointer here.
#[repr(C)]
struct Zone {
    private: [u8; 0],
}

unsafe extern "C" {
    fn malloc_create_zone(start_size: usize, flags: c_uint) -> *mut Zone;
    fn malloc_destroy_zone(zone: *mut Zone);
    fn malloc_zone_malloc(zone: *mut Zone, size: usize) -> *mut c_void;
    fn malloc_zone_calloc(zone: *mut Zone, count: usize, size: usize) -> *mut c_void;
    fn malloc_zone_memalign(zone: *mut Zone, align: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// What every block of malloc is aligned to on arm64.
const MALLOC_ALIGN: usize = 16;

static ZONE: AtomicPtr<Zone> = AtomicPtr::new(null_mut());

fn zone() -> *mut Zone {
    let zone = ZONE.load(Ordering::Acquire);
    if !zone.is_null() {
        return zone;
    }
    // SAFETY: plain calls of the system allocator.
    let made = unsafe { malloc_create_zone(0, 0) };
    match ZONE.compare_exchange(null_mut(), made, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => made,
        Err(first) => {
            // SAFETY: another thread was first, nothing is in this zone.
            unsafe { malloc_destroy_zone(made) };
            first
        }
    }
}

struct HotHeap;

// `free` and `realloc` of the system find the zone of a block by
// themselves. So a block that C code made can be freed here, and a block
// from here can be freed by C code, like with the heap of the process.
unsafe impl GlobalAlloc for HotHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the zone is alive until the loader frees it after the stop.
        unsafe {
            if layout.align() <= MALLOC_ALIGN {
                malloc_zone_malloc(zone(), layout.size()).cast()
            } else {
                malloc_zone_memalign(zone(), layout.align(), layout.size()).cast()
            }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as in `alloc`, and the block is `layout.size()` long.
        unsafe {
            if layout.align() <= MALLOC_ALIGN {
                return malloc_zone_calloc(zone(), 1, layout.size()).cast();
            }
            let block = self.alloc(layout);
            if !block.is_null() {
                block.write_bytes(0, layout.size());
            }
            block
        }
    }

    unsafe fn dealloc(&self, block: *mut u8, _layout: Layout) {
        // SAFETY: the caller gives a live block.
        unsafe { free(block.cast()) };
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller gives a live block of `layout`.
        unsafe {
            if layout.align() <= MALLOC_ALIGN {
                return realloc(block.cast(), new_size).cast();
            }
            let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
            let new_block = self.alloc(new_layout);
            if !new_block.is_null() {
                new_block.copy_from_nonoverlapping(block, layout.size().min(new_size));
                self.dealloc(block, layout);
            }
            new_block
        }
    }
}

#[global_allocator]
static HEAP: HotHeap = HotHeap;

/// The loader calls this some time after `hilen_stopped` said 1, when the
/// last threads of this build have ended. Every Rust allocation of the
/// build is gone after it, and no code of the build may run again.
#[unsafe(no_mangle)]
pub extern "C" fn hilen_free_heap() {
    let zone = ZONE.swap(null_mut(), Ordering::AcqRel);
    if !zone.is_null() {
        // SAFETY: the build is stopped and nothing of it runs.
        unsafe { malloc_destroy_zone(zone) };
    }
}
