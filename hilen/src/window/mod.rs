mod text;
mod window;
mod window_events;

mod app_handler;
mod frame_counter;
#[cfg(desktop)]
mod icon;
pub mod image;
mod placement;
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
    keyboard::NamedKey,
    window::Theme,
};

/// On wasm only the test suite reads the flag, the frame pacing that
/// reads it natively lives in a `not_wasm` block.
#[cfg(any(not_wasm, feature = "ui-tests"))]
pub(crate) use self::redraw::continuous_render_active;
#[cfg(not_wasm)]
pub(crate) use self::redraw::set_wake_proxy;
#[cfg(not_wasm)]
pub(crate) use self::redraw::take_needs_render;
pub use self::{
    app_handler::AppHandler,
    placement::*,
    render_frame::RenderFrame,
    screenshot::*,
    state::{msaa_sample_count, surface_texture_format},
    text::*,
    vertex_buffer::VertexBuffer,
    window::*,
    window_events::*,
};
pub(crate) use self::{app_handler::UserEvent, redraw::request_frame};
