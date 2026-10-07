mod text;
mod window;
mod window_events;

mod app_handler;
pub(crate) mod frame_control;
mod frame_counter;
#[cfg(feature = "inspect")]
pub(crate) mod frame_record;
#[cfg(feature = "inspect")]
pub(crate) mod frame_step;
mod fullscreen;
#[cfg(desktop)]
mod icon;
pub mod image;
mod msaa;
mod orientation;
mod placement;
#[cfg(desktop)]
pub(crate) mod placement_store;
mod redraw;
mod render_frame;
mod screen;
mod screenshot;
pub mod state;
mod surface;
mod vertex_buffer;
#[cfg(linux)]
pub(crate) mod wsl;

pub use bytemuck::cast_slice;
pub use wgpu::{
    Buffer, BufferUsages, Device, PolygonMode, RenderPass,
    util::{BufferInitDescriptor, DeviceExt},
};
pub use winit::{
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, NamedKey},
    window::Theme,
};

/// On wasm only the test suite reads the flag, the frame pacing that
/// reads it natively lives in a `not_wasm` block.
#[cfg(any(not_wasm, feature = "ui-tests"))]
pub(crate) use self::redraw::continuous_render_active;
#[cfg(not_wasm)]
pub(crate) use self::redraw::{
    frame_pacing, occluded, set_occluded, set_wake_proxy, take_needs_render, visibility,
};
pub use self::{
    app_handler::AppHandler, msaa::msaa_sample_count, orientation::Orientations, placement::*,
    render_frame::RenderFrame, screenshot::*, state::surface_texture_format, text::*,
    vertex_buffer::VertexBuffer, window::*, window_events::*,
};
pub(crate) use self::{
    app_handler::UserEvent,
    fullscreen::{reset_fullscreen, sync_fullscreen},
    orientation::reset_orientations,
    redraw::request_frame,
    render_frame::PassTarget,
};
