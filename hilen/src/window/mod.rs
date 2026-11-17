//! The window and what draws into it. With no `gpu` feature only the parts
//! the views reach stay: the window slots, the frame request, the
//! placement, the orientations and the screenshot type.

#[cfg(feature = "gpu")]
mod window;
#[cfg(feature = "gpu")]
mod window_events;
#[cfg(feature = "gpu")]
pub(crate) mod winit_keys;

#[cfg(feature = "gpu")]
mod app_handler;
#[cfg(feature = "gpu")]
mod extra;
#[cfg(all(macos, feature = "gpu"))]
mod extra_mac;
pub(crate) mod frame_control;
#[cfg(feature = "gpu")]
mod frame_counter;
#[cfg(feature = "inspect")]
pub(crate) mod frame_record;
#[cfg(feature = "inspect")]
pub(crate) mod frame_ring;
mod frame_state;
#[cfg(feature = "inspect")]
pub(crate) mod frame_step;
#[cfg(feature = "gpu")]
mod fullscreen;
#[cfg(feature = "gpu")]
mod gpu_capture;
#[cfg(all(desktop, feature = "gpu"))]
mod icon;
#[cfg(feature = "gpu")]
mod msaa;
mod orientation;
mod placement;
#[cfg(desktop)]
pub(crate) mod placement_store;
mod redraw;
#[cfg(feature = "gpu")]
mod render_frame;
#[cfg(feature = "gpu")]
mod screen;
mod screenshot;
pub(crate) mod slots;
#[cfg(feature = "gpu")]
pub mod state;
#[cfg(feature = "gpu")]
mod surface;
#[cfg(feature = "gpu")]
mod vertex_buffer;
#[cfg(all(linux, feature = "gpu"))]
pub(crate) mod wsl;

#[cfg(feature = "gpu")]
pub use bytemuck::cast_slice;
#[cfg(feature = "gpu")]
pub use wgpu::{
    Buffer, BufferUsages, Device, PolygonMode, RenderPass,
    util::{BufferInitDescriptor, DeviceExt},
};
#[cfg(feature = "gpu")]
pub use winit::window::Theme;

#[cfg(all(any(desktop, wasm), feature = "gpu"))]
pub(crate) use self::app_handler::ready_winit_window;
#[cfg(not(feature = "gpu"))]
pub use self::frame_state::Window;
#[cfg(feature = "inspect")]
pub(crate) use self::msaa::graphics_backend;
#[cfg(feature = "scene")]
pub(crate) use self::msaa::hides_covered_pixels;
#[cfg(all(wasm, feature = "gpu"))]
pub(crate) use self::redraw::needs_render;
pub(crate) use self::redraw::request_frame;
#[cfg(all(not_wasm, feature = "gpu"))]
pub(crate) use self::redraw::{
    Pacer, frame_drawn, frame_pacing, occluded, set_occluded, set_wake_proxy, visibility,
};
#[cfg(not(feature = "gpu"))]
pub(crate) use self::slots::open_slots;
#[cfg(feature = "gpu")]
pub use self::{
    app_handler::AppHandler,
    extra::{AppWindow, Display, WindowLevel, WindowPlace, WindowSpec},
    gpu_capture::GpuCapture,
    msaa::{msaa_sample_count, set_msaa_sample_count},
    render_frame::RenderFrame,
    state::surface_texture_format,
    vertex_buffer::VertexBuffer,
    window::*,
    window_events::*,
};
#[cfg(feature = "gpu")]
pub(crate) use self::{
    app_handler::UserEvent,
    extra::open_slots,
    fullscreen::{reset_fullscreen, sync_fullscreen},
    orientation::reset_orientations,
    redraw::{continuous_render_active, take_needs_render},
    render_frame::PassTarget,
};
pub use self::{orientation::Orientations, placement::*, screenshot::*};
