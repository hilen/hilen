//! The bind groups and layouts of `MeshPipeline`: the view, the
//! instances with the joints, the lights and the textures of a draw.

use std::num::NonZeroU64;

use indexmap::IndexMap;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType, Sampler,
    SamplerBindingType, ShaderStages, TextureSampleType, TextureView, TextureViewDimension,
};

use super::mesh_pipeline::{InstancesKey, MeshKey, TexturesKey};
use crate::{
    deps::refs::Weak,
    gm::{flat::Size, volume::Mat4},
    render::{
        bind_cache::{CachedBind, StorageKey, cached},
        data::MeshInstance,
        vec_buffer::VecBuffer,
    },
    scene::Sky,
    window::{
        Window,
        image::{Image, Texture, TextureRawData},
    },
};

/// The frame's view, the sky cube and the sun's shadow map, one layer
/// per cascade.
pub(super) fn view_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "scene_view_layout".into(),
        entries: &[
            BindGroupLayoutEntry {
                binding:    0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty:         BindingType::Buffer {
                    ty:                 BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count:      None,
            },
            texture_entry(1, TextureViewDimension::Cube),
            sampler_entry(2),
            BindGroupLayoutEntry {
                binding:    3,
                visibility: ShaderStages::FRAGMENT,
                ty:         BindingType::Texture {
                    multisampled:   false,
                    view_dimension: TextureViewDimension::D2Array,
                    sample_type:    TextureSampleType::Depth,
                },
                count:      None,
            },
        ],
    })
}

pub(super) fn view_bind(
    layout: &BindGroupLayout,
    view_buffer: &Buffer,
    sky: &Sky,
    shadow_map: &TextureView,
) -> BindGroup {
    Window::device().create_bind_group(&BindGroupDescriptor {
        label: Some("scene_view_bind"),
        layout,
        entries: &[
            BindGroupEntry {
                binding:  0,
                resource: view_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding:  1,
                resource: BindingResource::TextureView(&sky.view),
            },
            BindGroupEntry {
                binding:  2,
                resource: BindingResource::Sampler(&sky.sampler),
            },
            BindGroupEntry {
                binding:  3,
                resource: BindingResource::TextureView(shadow_map),
            },
        ],
    })
}

/// The instances for the fragment stage and the joints for the vertex
/// stage. Four bind groups is every lane's limit, so the two share one.
pub(super) fn instances_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "mesh_instances_layout".into(),
        entries: &[
            storage_entry(0, ShaderStages::FRAGMENT),
            storage_entry(1, ShaderStages::VERTEX),
        ],
    })
}

/// The part of the instance buffer this frame's flush landed in, so the
/// fragment stage indexes the same elements the draw uses, and the
/// frame's joints for the vertex stage.
pub(super) fn instances_bind(
    layout: &BindGroupLayout,
    instances: &VecBuffer<MeshInstance>,
    joints: &VecBuffer<Mat4>,
) -> BindGroup {
    Window::device().create_bind_group(&BindGroupDescriptor {
        label: Some("mesh_instances_bind"),
        layout,
        entries: &[
            BindGroupEntry {
                binding:  0,
                resource: BindingResource::Buffer(loaded_range(instances)),
            },
            BindGroupEntry {
                binding:  1,
                resource: BindingResource::Buffer(loaded_range(joints)),
            },
        ],
    })
}

/// The bind over the opaque instances and the joints of the frame, what
/// the frame's pass and the shadow passes both draw with.
pub(super) fn opaque_bind<'a>(
    slot: &'a mut Option<CachedBind<InstancesKey>>,
    layout: &BindGroupLayout,
    instances: &VecBuffer<MeshInstance>,
    joints: &VecBuffer<Mat4>,
) -> &'a BindGroup {
    cached(slot, (StorageKey::of(instances), StorageKey::of(joints)), || {
        instances_bind(layout, instances, joints)
    })
}

/// The base color texture and the normal map of a draw.
pub(super) fn textures_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "mesh_textures_layout".into(),
        entries: &[
            texture_entry(0, TextureViewDimension::D2),
            sampler_entry(1),
            texture_entry(2, TextureViewDimension::D2),
            sampler_entry(3),
        ],
    })
}

pub(super) fn storage_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Buffer {
            ty:                 BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size:   None,
        },
        count: None,
    }
}

pub(super) fn texture_entry(binding: u32, view_dimension: TextureViewDimension) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::FRAGMENT,
        ty: BindingType::Texture {
            multisampled: false,
            view_dimension,
            sample_type: TextureSampleType::Float { filterable: true },
        },
        count: None,
    }
}

pub(super) fn sampler_entry(binding: u32) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::FRAGMENT,
        ty: BindingType::Sampler(SamplerBindingType::Filtering),
        count: None,
    }
}

pub(super) fn plain_texture(rgba: [u8; 4], label: &str) -> Texture {
    Texture::from_raw_data(
        TextureRawData {
            data:     rgba.to_vec(),
            size:     Size::new(1, 1),
            channels: 4,
        },
        label,
    )
}

/// The image's texture, or the plain one when the material has none or
/// the image is gone.
pub(super) fn image_or<'a>(
    image: Option<&'a Weak<Image>>,
    plain: &'a Texture,
) -> (&'a TextureView, &'a Sampler) {
    match image {
        Some(image) if image.is_ok() => {
            let image: &'a Image = image;
            (image.view(), image.sampler())
        }
        _ => (&plain.view, &plain.sampler),
    }
}

/// The textures bind of `key` out of `cache`, remade when one of its
/// images has died since, its plain stand in is bound then.
pub(super) fn textures_bind<'a>(
    cache: &'a mut IndexMap<MeshKey, Option<CachedBind<TexturesKey>>>,
    layout: &BindGroupLayout,
    key: &MeshKey,
    white: &Texture,
    flat_normal: &Texture,
) -> &'a BindGroup {
    let alive = |image: &Option<Weak<Image>>| image.as_ref().is_some_and(Weak::is_ok);
    let slot = cache.entry(key.clone()).or_default();
    cached(slot, (alive(&key.texture), alive(&key.normal_map)), || {
        make_textures_bind(layout, key, white, flat_normal)
    })
}

pub(super) fn make_textures_bind(
    layout: &BindGroupLayout,
    key: &MeshKey,
    white: &Texture,
    flat_normal: &Texture,
) -> BindGroup {
    let (base_view, base_sampler) = image_or(key.texture.as_ref(), white);
    let (normal_view, normal_sampler) = image_or(key.normal_map.as_ref(), flat_normal);
    Window::device().create_bind_group(&BindGroupDescriptor {
        label: Some("mesh_textures_bind"),
        layout,
        entries: &[
            BindGroupEntry {
                binding:  0,
                resource: BindingResource::TextureView(base_view),
            },
            BindGroupEntry {
                binding:  1,
                resource: BindingResource::Sampler(base_sampler),
            },
            BindGroupEntry {
                binding:  2,
                resource: BindingResource::TextureView(normal_view),
            },
            BindGroupEntry {
                binding:  3,
                resource: BindingResource::Sampler(normal_sampler),
            },
        ],
    })
}

/// The part of a loaded `VecBuffer` that this frame's flush landed in.
pub(super) fn loaded_range<T>(buffer: &VecBuffer<T>) -> BufferBinding<'_> {
    let range = buffer.range();
    BufferBinding {
        buffer: buffer.buffer(),
        offset: range.start,
        size:   NonZeroU64::new(range.end - range.start),
    }
}

/// Binds the loaded part of a `VecBuffer` alone, so the shader indexes
/// the same elements the draw uses.
pub(super) fn storage_bind<T>(label: &str, layout: &BindGroupLayout, buffer: &VecBuffer<T>) -> BindGroup {
    Window::device().create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[BindGroupEntry {
            binding:  0,
            resource: BindingResource::Buffer(loaded_range(buffer)),
        }],
    })
}
