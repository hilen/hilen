mod text;
mod window;
mod window_events;

mod app_handler;
mod frame_counter;
pub mod image;
mod redraw;
mod render_frame;
mod screen;
mod screenshot;
pub mod state;
mod surface;
mod vertex_buffer;

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

#[cfg(not_wasm)]
pub(crate) use self::redraw::set_wake_proxy;
#[cfg(not_wasm)]
pub(crate) use self::redraw::take_needs_render;
pub use self::{
    app_handler::AppHandler, render_frame::RenderFrame, screenshot::*, state::SURFACE_TEXTURE_FORMAT,
    text::*, vertex_buffer::VertexBuffer, window::*, window_events::*,
};
pub(crate) use self::{
    app_handler::UserEvent,
    redraw::{continuous_render_active, request_frame},
};
