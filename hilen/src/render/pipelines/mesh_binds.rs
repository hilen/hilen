//! The bind groups and layouts of `MeshPipeline`: the view, the
//! instances with the joints, the lights and the textures of a draw.

use std::num::NonZeroU64;

use indexmap::IndexMap;
use log::warn;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType, Sampler,
    SamplerBindingType, ShaderStages, TextureSampleType, TextureView, TextureViewDimension,
};

use super::{
    air_textures::AirTextures,
    mesh_pipeline::{InstancesKey, MeshKey, TexturesKey},
    sky_view::SkyView,
};
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

/// The frame's view, the sky cube, the sun's shadow map, one layer per
/// cascade, the 2 tables of the air and the sky view table.
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
            texture_entry(4, TextureViewDimension::D2),
            texture_entry(5, TextureViewDimension::D2),
            texture_entry(6, TextureViewDimension::D2),
            texture_entry(7, TextureViewDimension::D2),
            sampler_entry(8),
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
    air: &AirTextures,
    sky_view: &SkyView,
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
            BindGroupEntry {
                binding:  4,
                resource: BindingResource::TextureView(&air.transmittance),
            },
            BindGroupEntry {
                binding:  5,
                resource: BindingResource::TextureView(&air.multi),
            },
            BindGroupEntry {
                binding:  6,
                resource: BindingResource::TextureView(&sky_view.light),
            },
            BindGroupEntry {
                binding:  7,
                resource: BindingResource::TextureView(&sky_view.through),
            },
            BindGroupEntry {
                binding:  8,
                resource: BindingResource::Sampler(&sky_view.sampler),
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

/// The base color texture, the normal map, the metallic roughness map
/// and the occlusion map of a draw. With the sky and the shadow map the
/// fragment stage reads 6 textures through 5 samplers, under the 16 of
/// WebGL2 and of an A7.
pub(super) fn textures_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "mesh_textures_layout".into(),
        entries: &[
            texture_entry(0, TextureViewDimension::D2),
            sampler_entry(1),
            texture_entry(2, TextureViewDimension::D2),
            sampler_entry(3),
            texture_entry(4, TextureViewDimension::D2),
            sampler_entry(5),
            texture_entry(6, TextureViewDimension::D2),
            sampler_entry(7),
            texture_entry(8, TextureViewDimension::D2),
            sampler_entry(9),
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

/// The images of a draw's material, all its textures bind holds. Meshes
/// with the same images share 1 bind, every mesh with no maps the plain one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct TextureSet {
    texture:                Option<Weak<Image>>,
    normal_map:             Option<Weak<Image>>,
    metallic_roughness_map: Option<Weak<Image>>,
    occlusion_map:          Option<Weak<Image>>,
    flow_map:               Option<Weak<Image>>,
}

impl TextureSet {
    fn of(key: &MeshKey) -> Self {
        Self {
            texture:                key.texture,
            normal_map:             key.normal_map,
            metallic_roughness_map: key.metallic_roughness_map,
            occlusion_map:          key.occlusion_map,
            flow_map:               key.flow_map,
        }
    }

    fn images(&self) -> [&Option<Weak<Image>>; 5] {
        [
            &self.texture,
            &self.normal_map,
            &self.metallic_roughness_map,
            &self.occlusion_map,
            &self.flow_map,
        ]
    }

    /// Whether every image it names still lives. One that died is bound as
    /// its plain stand in from then on, so its set is dropped.
    pub(super) fn alive(&self) -> bool {
        self.images().iter().all(|image| image.as_ref().is_none_or(Weak::is_ok))
    }
}

/// The bind of a set and the frame its images were last checked on.
#[derive(Default)]
pub(super) struct TexturesEntry {
    bind:    Option<CachedBind<TexturesKey>>,
    checked: u64,
}

/// Where the textures bind of `key` is in `cache`, made or remade when one
/// of its images died since. Its images are checked once a `frame`, each
/// draw then reads the bind by the index with no check, see `textures_at`.
pub(super) fn textures_index(
    cache: &mut IndexMap<TextureSet, TexturesEntry>,
    layout: &BindGroupLayout,
    key: &MeshKey,
    plain: (&Texture, &Texture),
    frame: u64,
) -> usize {
    let entry = cache.entry(TextureSet::of(key));
    let index = entry.index();
    entry.or_default();
    let (set, slot) = cache.get_index_mut(index).expect("just put in");
    if slot.checked != frame || slot.bind.is_none() {
        let alive = |image: &Option<Weak<Image>>| image.as_ref().is_some_and(Weak::is_ok);
        let alive_now = (
            alive(&set.texture),
            alive(&set.normal_map),
            alive(&set.metallic_roughness_map),
            alive(&set.occlusion_map),
            alive(&set.flow_map),
        );
        cached(&mut slot.bind, alive_now, || {
            make_textures_bind(layout, set, plain.0, plain.1)
        });
        slot.checked = frame;
    }
    index
}

/// The bind `textures_index` gave `index` this frame.
pub(super) fn textures_at(cache: &IndexMap<TextureSet, TexturesEntry>, index: usize) -> &BindGroup {
    cache[index].bind.as_ref().expect("made by textures_index").bind()
}

/// The 2 materials a mesh cannot show the way they are set, said once per
/// mesh, see `MeshPipeline::add`.
pub(super) fn warn_unshown(key: &MeshKey) -> bool {
    let maps = key.texture.is_some()
        || key.normal_map.is_some()
        || key.metallic_roughness_map.is_some()
        || key.occlusion_map.is_some();
    if key.mesh.smooth_colors && maps {
        warn!("a mesh with fading colors has no uv, the maps of its material show only their first texel");
        return true;
    }
    if key.mesh.flowing && key.cutout {
        warn!("a mesh with a flow cannot be cut out, its flow reads as a color there");
        return true;
    }
    false
}

fn make_textures_bind(
    layout: &BindGroupLayout,
    key: &TextureSet,
    white: &Texture,
    flat_normal: &Texture,
) -> BindGroup {
    let (base_view, base_sampler) = image_or(key.texture.as_ref(), white);
    let (normal_view, normal_sampler) = image_or(key.normal_map.as_ref(), flat_normal);
    let (metallic_roughness_view, metallic_roughness_sampler) =
        image_or(key.metallic_roughness_map.as_ref(), white);
    let (occlusion_view, occlusion_sampler) = image_or(key.occlusion_map.as_ref(), white);
    // Half in red and green is no flow, like a flat normal.
    let (flow_view, flow_sampler) = image_or(key.flow_map.as_ref(), flat_normal);
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
            BindGroupEntry {
                binding:  4,
                resource: BindingResource::TextureView(metallic_roughness_view),
            },
            BindGroupEntry {
                binding:  5,
                resource: BindingResource::Sampler(metallic_roughness_sampler),
            },
            BindGroupEntry {
                binding:  6,
                resource: BindingResource::TextureView(occlusion_view),
            },
            BindGroupEntry {
                binding:  7,
                resource: BindingResource::Sampler(occlusion_sampler),
            },
            BindGroupEntry {
                binding:  8,
                resource: BindingResource::TextureView(flow_view),
            },
            BindGroupEntry {
                binding:  9,
                resource: BindingResource::Sampler(flow_sampler),
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
