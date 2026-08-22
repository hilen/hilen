use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, ShaderStages};

use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        color::Color,
        flat::{Point, Size},
    },
    render::{buffer_helper::BufferHelper, device_helper::DeviceHelper, uniform::make_uniform_layout},
    window::Window,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
struct PathView {
    position:   Point,
    resolution: Size,
    color:      Color,
    z_position: f32,
    scale:      f32,
    _padding:   [u32; 2],
}

/// One tessellated path on the GPU: an indexed triangle mesh in the
/// owning view's coordinate space plus the uniform placing it on
/// screen. The mesh is built once, `prepare` refreshes the uniform when
/// the view moves, the window resizes or the scale changes.
#[derive(Debug)]
pub struct PathData {
    view:          PathView,
    view_buffer:   Buffer,
    bind:          BindGroup,
    vertex_buffer: Buffer,
    index_buffer:  Buffer,
    index_count:   u32,
}

impl PathData {
    pub fn new(color: Color, vertices: &[Point], indices: &[u32]) -> Self {
        let device = Window::device();

        let vertex_buffer = device.buffer(vertices, BufferUsages::VERTEX);
        let index_buffer = device.buffer(indices, BufferUsages::INDEX);

        let view = PathView {
            position: Point::default(),
            resolution: Size::default(),
            color,
            z_position: 0.0,
            scale: 1.0,
            _padding: [0; 2],
        };

        let view_buffer = device.buffer(&view, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let bind = device.bind(&view_buffer, Self::uniform_layout());

        Self {
            view,
            view_buffer,
            bind,
            vertex_buffer,
            index_buffer,
            index_count: u32::try_from(indices.len()).unwrap(),
        }
    }

    /// Compares against the last uploaded state and writes the uniform
    /// only when something moved, so a static path costs nothing per
    /// frame.
    pub fn prepare(&mut self, position: Point, resolution: Size, scale: f32, z_position: f32) {
        let view = PathView {
            position,
            resolution,
            color: self.view.color,
            z_position,
            scale,
            _padding: [0; 2],
        };

        if view == self.view {
            return;
        }

        self.view = view;
        self.view_buffer.update(view);
    }

    /// A fully transparent path must not be drawn at all. Its fragments
    /// would still write depth and mask whatever draws behind them
    /// later, the same trap the rect shader avoids with its alpha
    /// discard.
    pub(crate) fn visible(&self) -> bool {
        self.view.color.a >= 0.004
    }

    pub(crate) fn uniform_bind(&self) -> &BindGroup {
        &self.bind
    }

    pub(crate) fn vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }

    pub(crate) fn index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }

    pub(crate) fn index_count(&self) -> u32 {
        self.index_count
    }

    pub(crate) fn uniform_layout() -> &'static BindGroupLayout {
        static LAYOUT: MainLock<BindGroupLayout> = MainLock::new();
        LAYOUT.get_or_init(|| make_uniform_layout("path_view_layout", ShaderStages::VERTEX_FRAGMENT))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<PathView>() % 16, 0);
    }
}
