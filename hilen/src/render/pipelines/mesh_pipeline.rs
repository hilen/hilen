use std::num::NonZeroU64;

use indexmap::IndexMap;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, IndexFormat, PipelineLayoutDescriptor, RenderPass, RenderPipeline, Sampler,
    SamplerBindingType, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension,
};

use crate::{
    deps::refs::Weak,
    gm::{flat::Size, volume::Vertex3D},
    render::{
        SceneView,
        buffer_helper::BufferHelper,
        data::{MeshInstance, MeshLight},
        device_helper::DeviceHelper,
        uniform::make_storage_layout,
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    scene::{Mesh, Sky},
    window::{
        Window,
        image::{Image, Texture, TextureRawData},
    },
};

/// What the mesh and the sky shaders share, prepended to both.
const COMMON: &str = include_str!("shaders/scene_common.wgsl");
const MESH: &str = include_str!("shaders/mesh.wgsl");
const SKY: &str = include_str!("shaders/sky.wgsl");

/// What one instanced draw shares: the mesh and the two textures.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeshKey {
    pub mesh:       Weak<Mesh>,
    pub texture:    Option<Weak<Image>>,
    pub normal_map: Option<Weak<Image>>,
}

/// Draws the sky and every node of a frame, one instanced indexed draw
/// per mesh and texture pair for the opaque nodes, then one draw per
/// translucent node, back to front.
pub struct MeshPipeline {
    opaque:      RenderPipeline,
    translucent: RenderPipeline,
    sky:         RenderPipeline,

    /// The frame's view, and the sky cube it lights and reflects.
    view_layout: BindGroupLayout,
    view_buffer: Buffer,
    /// Bound when the scene has no sky.
    black_sky:   Sky,

    /// Binds the instance buffer for the fragment stage, see
    /// `RectPipeline` for why the group cannot be cached.
    instances_layout: BindGroupLayout,

    // A unit mesh lives for the whole process, so a key cannot die. An
    // image can, then the draw falls back to the plain textures.
    instances: IndexMap<MeshKey, VecBuffer<MeshInstance>>,

    /// The translucent nodes in draw order and the key of each.
    transparent:      VecBuffer<MeshInstance>,
    transparent_keys: Vec<MeshKey>,

    /// The point and spot lights of the frame, indexed by every
    /// instance's light list.
    lights_layout: BindGroupLayout,
    lights:        VecBuffer<MeshLight>,

    /// The base color texture and the normal map of a draw, with the
    /// plain ones a material without them binds.
    textures_layout: BindGroupLayout,
    white:           Texture,
    flat_normal:     Texture,
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let device = Window::device();

        let mesh_shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  "mesh_shader".into(),
            source: ShaderSource::Wgsl(format!("{COMMON}\n{MESH}").into()),
        });
        let sky_shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  "sky_shader".into(),
            source: ShaderSource::Wgsl(format!("{COMMON}\n{SKY}").into()),
        });

        let view_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            ],
        });
        let instances_layout = make_storage_layout("mesh_instances_layout", ShaderStages::FRAGMENT);
        let lights_layout = make_storage_layout("mesh_lights_layout", ShaderStages::FRAGMENT);
        let textures_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label:   "mesh_textures_layout".into(),
            entries: &[
                texture_entry(0, TextureViewDimension::D2),
                sampler_entry(1),
                texture_entry(2, TextureViewDimension::D2),
                sampler_entry(3),
            ],
        });

        let mesh_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "mesh_pipeline_layout".into(),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(&instances_layout),
                Some(&lights_layout),
                Some(&textures_layout),
            ],
            immediate_size:     0,
        });
        let sky_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "sky_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout)],
            immediate_size:     0,
        });

        let layouts = &[Vertex3D::VERTEX_LAYOUT, MeshInstance::VERTEX_LAYOUT];
        let opaque = device.mesh_pipeline("mesh_pipeline", &mesh_layout, &mesh_shader, layouts, false);
        let translucent = device.mesh_pipeline(
            "mesh_translucent_pipeline",
            &mesh_layout,
            &mesh_shader,
            layouts,
            true,
        );
        let sky = device.sky_pipeline("sky_pipeline", &sky_layout, &sky_shader);

        let view_buffer = device.buffer(
            &SceneView::default(),
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        Self {
            opaque,
            translucent,
            sky,
            view_layout,
            view_buffer,
            black_sky: Sky::black(),
            instances_layout,
            instances: IndexMap::default(),
            transparent: VecBuffer::default(),
            transparent_keys: vec![],
            lights_layout,
            lights: VecBuffer::default(),
            textures_layout,
            white: plain_texture([255, 255, 255, 255], "mesh_white"),
            flat_normal: plain_texture([128, 128, 255, 255], "mesh_flat_normal"),
        }
    }
}

impl MeshPipeline {
    pub(crate) fn add(&mut self, key: MeshKey, mut instance: MeshInstance) {
        let batch = self.instances.entry(key).or_default();
        instance.index = batch.pending();
        batch.push(instance);
    }

    /// Translucent nodes draw in the order they are added, so add them
    /// back to front.
    pub(crate) fn add_transparent(&mut self, key: MeshKey, mut instance: MeshInstance) {
        instance.index = self.transparent.pending();
        self.transparent.push(instance);
        self.transparent_keys.push(key);
    }

    pub(crate) fn add_light(&mut self, light: MeshLight) {
        self.lights.push(light);
    }

    pub(crate) fn draw(&mut self, render_pass: &mut RenderPass, view: &SceneView, sky: Option<&Sky>) {
        let no_nodes = self.instances.values().all(VecBuffer::is_empty) && self.transparent.is_empty();
        if no_nodes && sky.is_none() {
            return;
        }

        self.view_buffer.update(*view);
        let view_bind = self.view_bind(sky.unwrap_or(&self.black_sky));
        render_pass.set_bind_group(0, &view_bind, &[]);

        if sky.is_some() {
            render_pass.set_pipeline(&self.sky);
            render_pass.draw(0..3, 0..1);
        }

        if no_nodes {
            return;
        }

        // A storage binding cannot be empty, and no instance indexes the
        // placeholder since every light count is zero then.
        if self.lights.is_empty() {
            self.lights.push(MeshLight::default());
        }
        self.lights.load();
        let lights_bind = storage_bind("mesh_lights_bind", &self.lights_layout, &self.lights);
        render_pass.set_bind_group(2, &lights_bind, &[]);

        render_pass.set_pipeline(&self.opaque);

        for (key, instances) in &mut self.instances {
            if instances.is_empty() {
                continue;
            }

            instances.load();

            let instances_bind = storage_bind("mesh_instances_bind", &self.instances_layout, instances);
            let textures_bind = textures_bind(&self.textures_layout, key, &self.white, &self.flat_normal);

            render_pass.set_bind_group(1, &instances_bind, &[]);
            render_pass.set_bind_group(3, &textures_bind, &[]);
            render_pass.set_vertex_buffer(0, key.mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice());
            render_pass.set_index_buffer(key.mesh.index_buffer.slice(..), IndexFormat::Uint16);

            // Base vertex stays zero, an A7 draws nothing otherwise.
            render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..instances.len());
        }

        if self.transparent.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.translucent);

        self.transparent.load();
        let instances_bind = storage_bind("mesh_translucent_bind", &self.instances_layout, &self.transparent);
        render_pass.set_bind_group(1, &instances_bind, &[]);

        let range = self.transparent.range().clone();
        let stride = MeshInstance::VERTEX_LAYOUT.array_stride;

        for (i, key) in self.transparent_keys.drain(..).enumerate() {
            let textures_bind = textures_bind(&self.textures_layout, &key, &self.white, &self.flat_normal);
            render_pass.set_bind_group(3, &textures_bind, &[]);
            render_pass.set_vertex_buffer(0, key.mesh.vertex_buffer.slice(..));
            // The draw starts at instance zero of a slice that begins at
            // this node, its `index` attribute still names the real slot.
            let start = range.start + u64::try_from(i).expect("node count fits u64") * stride;
            render_pass.set_vertex_buffer(1, self.transparent.buffer().slice(start..range.end));
            render_pass.set_index_buffer(key.mesh.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..1);
        }
    }

    fn view_bind(&self, sky: &Sky) -> BindGroup {
        Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("scene_view_bind"),
            layout:  &self.view_layout,
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: self.view_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::TextureView(&sky.view),
                },
                BindGroupEntry {
                    binding:  2,
                    resource: BindingResource::Sampler(&sky.sampler),
                },
            ],
        })
    }
}

fn texture_entry(binding: u32, view_dimension: TextureViewDimension) -> BindGroupLayoutEntry {
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

fn sampler_entry(binding: u32) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::FRAGMENT,
        ty: BindingType::Sampler(SamplerBindingType::Filtering),
        count: None,
    }
}

fn plain_texture(rgba: [u8; 4], label: &str) -> Texture {
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
fn image_or<'a>(image: Option<&'a Weak<Image>>, plain: &'a Texture) -> (&'a TextureView, &'a Sampler) {
    match image {
        Some(image) if image.is_ok() => {
            let image: &'a Image = image;
            (image.view(), image.sampler())
        }
        _ => (&plain.view, &plain.sampler),
    }
}

fn textures_bind(
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

/// Binds the part of a loaded `VecBuffer` that this frame's flush landed
/// in, so the shader indexes the same elements the draw uses.
fn storage_bind<T>(label: &str, layout: &BindGroupLayout, buffer: &VecBuffer<T>) -> BindGroup {
    let range = buffer.range();
    let buffer: &Buffer = buffer.buffer();
    Window::device().create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[BindGroupEntry {
            binding:  0,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset: range.start,
                size: NonZeroU64::new(range.end - range.start),
            }),
        }],
    })
}
