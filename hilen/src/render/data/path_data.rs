use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, ShaderStages};

use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        color::Color,
        flat::{MAX_STOPS, Paint, Point, Ramp, Size},
    },
    render::{buffer_helper::BufferHelper, device_helper::DeviceHelper, uniform::make_uniform_layout},
    window::Window,
};

const KIND_FLAT: u32 = 0;
const KIND_LINEAR: u32 = 1;
const KIND_RADIAL: u32 = 2;
const KIND_CONIC: u32 = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
struct PathView {
    position:   Point,
    resolution: Size,
    /// The linear start, the radial or conic center.
    gradient_a: Point,
    /// The linear end. For a radial `x` is the radius, for a conic the
    /// repeats per turn.
    gradient_b: Point,
    colors:     [Color; MAX_STOPS],
    /// The stop positions, packed as two vec4 so std140 does not pad
    /// each float to 16 bytes.
    positions:  [f32; MAX_STOPS],
    z_position: f32,
    scale:      f32,
    kind:       u32,
    stop_count: u32,
    grain:      f32,
    _padding:   [u32; 3],
}

/// One tessellated path on the GPU: an indexed triangle mesh in the
/// owning view's coordinate space plus the uniform placing it on
/// screen. The mesh is built once, `prepare` refreshes the uniform when
/// the view moves, the window resizes or the scale changes.
#[derive(Debug)]
pub struct PathData {
    visible:       bool,
    view:          PathView,
    view_buffer:   Buffer,
    bind:          BindGroup,
    vertex_buffer: Buffer,
    index_buffer:  Buffer,
    index_count:   u32,
}

impl PathData {
    pub fn new(paint: Paint, vertices: &[Point], indices: &[u32]) -> Self {
        let device = Window::device();

        let vertex_buffer = device.buffer(vertices, BufferUsages::VERTEX);
        let index_buffer = device.buffer(indices, BufferUsages::INDEX);

        let (gradient_a, gradient_b, kind) = match paint.ramp {
            Ramp::Flat => (Point::default(), Point::default(), KIND_FLAT),
            Ramp::Linear { from, to } => (from, to, KIND_LINEAR),
            Ramp::Radial { at, radius } => (at, Point::new(radius, 0.0), KIND_RADIAL),
            Ramp::Conic { at, repeats } => (at, Point::new(repeats, 0.0), KIND_CONIC),
        };

        let mut colors = [Color::default(); MAX_STOPS];
        let mut positions = [0.0; MAX_STOPS];
        for (i, (color, position)) in paint.stops[..paint.count].iter().enumerate() {
            colors[i] = *color;
            positions[i] = *position;
        }

        let view = PathView {
            position: Point::default(),
            resolution: Size::default(),
            gradient_a,
            gradient_b,
            colors,
            positions,
            z_position: 0.0,
            scale: 1.0,
            kind,
            stop_count: u32::try_from(paint.count).unwrap(),
            grain: paint.grain,
            _padding: [0; 3],
        };

        let view_buffer = device.buffer(&view, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let bind = device.bind(&view_buffer, Self::uniform_layout());

        Self {
            visible: paint.visible(),
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
            z_position,
            scale,
            ..self.view
        };

        if view == self.view {
            return;
        }

        self.view = view;
        self.view_buffer.update(view);
    }

    /// A fully transparent path must not be drawn at all. Its fragments
    /// would still write depth and mask whatever draws behind it
    /// later, the same trap the rect shader avoids with its alpha
    /// discard.
    pub(crate) fn visible(&self) -> bool {
        self.visible
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
    use std::mem::offset_of;

    use super::*;

    #[test]
    fn test() {
        // Web requirements, and the wgsl struct in ui_path.wgsl pins
        // these offsets.
        assert_eq!(size_of::<PathView>() % 16, 0);
        assert_eq!(offset_of!(PathView, gradient_a), 16);
        assert_eq!(offset_of!(PathView, gradient_b), 24);
        assert_eq!(offset_of!(PathView, colors), 32);
        assert_eq!(offset_of!(PathView, positions), 160);
        assert_eq!(offset_of!(PathView, z_position), 192);
        assert_eq!(offset_of!(PathView, kind), 200);
        assert_eq!(offset_of!(PathView, grain), 208);
    }
}
