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
mod code_meshes;
/// The wireframe lines, see `colliders`.
#[cfg(not_wasm)]
mod collider_shapes;
/// `SwiftShader` draws the green wireframe lines in another shade, see
/// docs/roadmap.md.
#[cfg(not_wasm)]
mod colliders;
mod cutout;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod cutout_shadows;
mod day_and_night;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod fade_shadows;
/// The same bits as on desktop in the browser, the picture fails on
/// `SwiftShader`, see docs/roadmap.md.
#[cfg(not_wasm)]
mod fog;
mod geometry;
mod glow;
mod grass;
mod grass_on_a_planet;
mod leaves;
mod lights;
mod materials;
mod metal_roughness;
mod mipped_picture;
mod models;
mod moon_dust;
mod moonlight;
/// Mouse capture needs a mouse, and a browser grants pointer lock only
/// after a real click, which no test can inject.
#[cfg(desktop)]
mod mouse_look;
mod near_and_far;
mod night_and_eclipse;
mod node_fade;
mod node_parenting;
mod orthographic_camera;
mod picking;
mod planet_sky;
mod planet_walk;
/// The same bits as on desktop in the browser, the picture fails on
/// `SwiftShader`, see docs/roadmap.md.
#[cfg(not_wasm)]
mod player_walk;
mod primitives;
mod ride_with_player;
mod scene_in_view;
mod scene_leaves_keys;
mod scene_lines;
mod scene_photo;
mod scene_picture;
mod scene_picture_clear;
mod scene_queries;
mod sea_under_air;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod shadow_out_of_view;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod shadows;
mod skybox;
mod sun_glare;
/// Shadow edges on `SwiftShader`, see `animations`.
#[cfg(not_wasm)]
mod terrain_shadows;
mod textures;
mod third_person_camera;
mod transform_handle;
mod transparency;
mod triangle_picking;
mod unlit;
mod vertex_colors;
mod view_points;
/// The white edge lines, see `colliders`.
#[cfg(not_wasm)]
mod wireframe;
mod world_move;

/// Names this crate so a linker keeps it.
///
/// Every test here registers through a `ctor` and nothing calls it by name, so
/// a linker drops the whole rlib and takes every test with it. Nothing reports
/// that, the suite just quietly runs fewer tests. A consumer must call this.
pub fn keep_linked() {
    std::hint::black_box(());
}
