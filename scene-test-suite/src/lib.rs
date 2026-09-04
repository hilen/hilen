//! The scene tests.
//!
//! A library rather than part of the `scene-test` binary so `demo` can
//! link it and carry every scene test onto a device, the way
//! `ui-test-suite` carries the UI tests. It must never depend on `demo`,
//! that would be a cycle, since the `scene-test` runner links both.
//!
//! Nothing here is called by name. Every test registers itself into
//! `hilen::SCENE_TESTS` through a `ctor`, so a consumer only has to keep
//! this crate linked. See `keep_linked`.

#![allow(incomplete_features)]
#![feature(specialization)]

/// Shadow edges land elsewhere on `SwiftShader`, the software WebGPU the CI
/// browser lane renders with, see docs/roadmap.md.
#[cfg(not_wasm)]
mod animations;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod cascades;
/// The physics rest lands elsewhere under the browser and simulator frame
/// pacing, see docs/roadmap.md.
#[cfg(desktop)]
mod colliders;
/// The physics rest, see `colliders`.
#[cfg(desktop)]
mod drop_balls;
mod fog;
mod lights;
mod materials;
mod models;
/// Mouse capture needs a mouse, and a browser grants pointer lock only
/// after a real click, which no test can inject.
#[cfg(desktop)]
mod mouse_look;
mod picking;
/// The physics rest, see `colliders`.
#[cfg(desktop)]
mod player_walk;
mod primitives;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod shadows;
mod skybox;
mod textures;
mod transparency;

/// Names this crate so a linker keeps it.
///
/// Every test here registers through a `ctor` and nothing calls it by name, so
/// a linker drops the whole rlib and takes every test with it. Nothing reports
/// that, the suite just quietly runs fewer tests. A consumer must call this.
pub fn keep_linked() {
    std::hint::black_box(());
}
