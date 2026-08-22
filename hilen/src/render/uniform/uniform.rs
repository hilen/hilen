use bytemuck::Pod;
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, ShaderStages,
};

use crate::{
    render::device_helper::DeviceHelper,
    window::{BufferUsages, Window},
};

pub(crate) fn make_bind<T: Pod>(data: &T, layout: &BindGroupLayout) -> BindGroup {
    let device = Window::device();
    let buffer = device.buffer(data, BufferUsages::UNIFORM);
    device.bind(&buffer, layout)
}

pub(crate) fn make_uniform_layout(name: &str, shader: ShaderStages) -> BindGroupLayout {
    Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   name.into(),
        entries: &[BindGroupLayoutEntry {
            binding:    0,
            visibility: shader,
            ty:         BindingType::Buffer {
                ty:                 BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size:   None,
            },
            count:      None,
        }],
    })
}

/// Read only instance data for the fragment stage. Everything a rect shader
/// needs per instance is constant across the shape, so carrying it as an inter
/// stage varying costs a payload that an A7 GPU silently refuses to draw, see
/// `docs/ios.md`. The fragment reads it from here instead and only `uv` and the
/// instance index cross the stage boundary.
pub(crate) fn make_storage_layout(name: &str, shader: ShaderStages) -> BindGroupLayout {
    Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   name.into(),
        entries: &[BindGroupLayoutEntry {
            binding:    0,
            visibility: shader,
            ty:         BindingType::Buffer {
                ty:                 BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size:   None,
            },
            count:      None,
        }],
    })
}
