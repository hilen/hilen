//! The UI test corpus.
//!
//! A library rather than part of the `ui-test` binary so `demo` can link
//! it and carry every test onto a device. It must never depend on `demo`,
//! that would be a cycle, since the `ui-test` runner links both.
//!
//! Nothing here is called by name. Every test registers itself into
//! `hilen::UI_TESTS` through a `ctor`, so a consumer only has to keep
//! this crate linked. See `keep_linked`.

#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

mod base;
// The engine compiles the inspector out on wasm, so its tests go with it.
#[cfg(not_wasm)]
mod inspect;
mod level;
mod views;

/// Names this crate so a linker keeps it.
///
/// Every test here registers through a `ctor` and nothing calls it by name, so
/// a linker drops the whole rlib and takes the corpus with it. Nothing reports
/// that, the suite just quietly runs fewer tests. A consumer must call this.
pub fn keep_linked() {
    std::hint::black_box(());
}
