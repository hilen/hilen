use std::num::NonZeroU64;

use indexmap::IndexMap;
use plat::Platform;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, CommandEncoder, IndexFormat, PipelineLayoutDescriptor, RenderPass, RenderPipeline, Sampler,
    SamplerBindingType, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension,
};

use crate::{
    deps::refs::Weak,
    gm::{
        flat::Size,
        volume::{Mat4, SkinVertex, Vertex3D},
    },
    render::{
        SHADOW_CASCADES, SceneView,
        buffer_helper::BufferHelper,
        data::{MeshInstance, MeshLight},
        device_helper::DeviceHelper,
        pipelines::shadow_pass::ShadowPass,
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
const SHADOW: &str = include_str!("shaders/shadow.wgsl");

/// The vertex buffers of a static draw and of a skinned one, which adds
/// the joints and weights per vertex.
pub(super) const STATIC_LAYOUTS: &[wgpu::VertexBufferLayout] =
    &[Vertex3D::VERTEX_LAYOUT, MeshInstance::VERTEX_LAYOUT];
pub(super) const SKINNED_LAYOUTS: &[wgpu::VertexBufferLayout] = &[
    Vertex3D::VERTEX_LAYOUT,
    MeshInstance::VERTEX_LAYOUT,
    SkinVertex::VERTEX_LAYOUT,
];

/// What one instanced draw shares: the mesh and the two textures.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeshKey {
    pub mesh:       Weak<Mesh>,
    pub texture:    Option<Weak<Image>>,
    pub normal_map: Option<Weak<Image>>,
}

/// Draws the sky and every node of a frame, one instanced indexed draw
/// per mesh and texture pair for the opaque nodes, then one draw per
/// translucent node, back to front. `prepare` loads the frame's buffers
/// and draws the shadow map before the frame's pass opens, `draw` draws
/// into it. A skinned mesh draws through the skinned twin of each
/// pipeline, the same shader from its other vertex entry, which blends
/// the joint matrices its instance points at.
pub struct MeshPipeline {
    opaque:              RenderPipeline,
    opaque_skinned:      RenderPipeline,
    translucent:         RenderPipeline,
    translucent_skinned: RenderPipeline,
    sky:                 RenderPipeline,

    /// The frame's view, the sky cube it lights and reflects, and the
    /// sun's shadow map.
    view_layout: BindGroupLayout,
    view_buffer: Buffer,
    /// Bound when the scene has no sky.
    black_sky:   Sky,

    /// The sun's depth passes, which cannot bind the map they draw, so
    /// they carry the light's matrices on their own.
    shadow: ShadowPass,

    /// Binds the instance buffer for the fragment stage and the joint
    /// buffer for the vertex stage, see `RectPipeline` for why the
    /// group cannot be cached.
    instances_layout: BindGroupLayout,

    // A unit mesh lives for the whole process, a model's mesh dies with
    // the model and takes its key out at the next draw. An image can die
    // too, then the draw falls back to the plain textures.
    instances: IndexMap<MeshKey, VecBuffer<MeshInstance>>,

    /// The translucent nodes in draw order and the key of each.
    transparent:      VecBuffer<MeshInstance>,
    transparent_keys: Vec<MeshKey>,

    /// The joint matrices of every skinned node of the frame, one run
    /// per node and skin, indexed by the instance's `joint_base`.
    joints: VecBuffer<Mat4>,

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

impl MeshPipeline {
    /// Texels along each side of every cascade of the shadow map. A
    /// phone and a browser get a quarter of the desktop's area.
    pub(crate) const SHADOW_MAP_SIZE: u32 = if Platform::MOBILE || Platform::WASM {
        1024
    } else {
        2048
    };
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let device = Window::device();

        // The common part reads the cascade count, one source for both.
        let common = format!("const SHADOW_CASCADES: i32 = {SHADOW_CASCADES};\n{COMMON}");
        let mesh_shader = shader(device, "mesh_shader", format!("{common}\n{MESH}"));
        let sky_shader = shader(device, "sky_shader", format!("{common}\n{SKY}"));
        let shadow_shader = shader(device, "shadow_shader", SHADOW.to_string());

        let view_layout = view_layout(device);
        let instances_layout = instances_layout(device);
        let lights_layout = make_storage_layout("mesh_lights_layout", ShaderStages::FRAGMENT);
        let textures_layout = textures_layout(device);

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

        let [opaque, opaque_skinned] = mesh_pipelines(device, "mesh", &mesh_layout, &mesh_shader, false);
        let [translucent, translucent_skinned] =
            mesh_pipelines(device, "mesh_translucent", &mesh_layout, &mesh_shader, true);
        let sky = device.sky_pipeline("sky_pipeline", &sky_layout, &sky_shader);

        let view_buffer = device.buffer(
            &SceneView::default(),
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        Self {
            opaque,
            opaque_skinned,
            translucent,
            translucent_skinned,
            sky,
            view_layout,
            view_buffer,
            black_sky: Sky::black(),
            shadow: ShadowPass::new(device, &shadow_shader, Self::SHADOW_MAP_SIZE),
            instances_layout,
            instances: IndexMap::default(),
            transparent: VecBuffer::default(),
            transparent_keys: vec![],
            joints: VecBuffer::default(),
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

    /// Queues the joint matrices of one skinned node and returns where
    /// they start, the `joint_base` of its instances.
    pub(crate) fn add_joints(&mut self, matrices: &[Mat4]) -> u32 {
        let base = self.joints.pending();
        for matrix in matrices {
            self.joints.push(*matrix);
        }
        base
    }

    /// Loads the frame's buffers and, when the sun casts, draws the
    /// shadow map's cascades, `map_size` texels a side. Before the
    /// frame's pass opens, the pass reads the map.
    pub(crate) fn prepare(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &SceneView,
        shadows: bool,
        map_size: u32,
    ) {
        self.instances.retain(|key, _| key.mesh.is_ok());

        self.view_buffer.update(*view);

        // A storage binding cannot be empty, and no instance indexes the
        // placeholder since every light count is zero then.
        if self.lights.is_empty() {
            self.lights.push(MeshLight::default());
        }
        self.lights.load();

        // The same for the joints, only a skinned draw reads them.
        if self.joints.is_empty() {
            self.joints.push(Mat4::IDENTITY);
        }
        self.joints.load();

        for instances in self.instances.values_mut() {
            if !instances.is_empty() {
                instances.load();
            }
        }
        if !self.transparent.is_empty() {
            self.transparent.load();
        }

        if shadows {
            self.shadow.fit(Window::device(), map_size);
            let batches: Vec<_> = self.loaded().collect();
            self.shadow.draw(encoder, &view.sun_view_proj, &batches, &self.joints);
        }
    }

    /// The batches `prepare` loaded this frame.
    fn loaded(&self) -> impl Iterator<Item = (&MeshKey, &VecBuffer<MeshInstance>)> {
        let frame = Window::render_frame();
        self.instances
            .iter()
            .filter(move |(_, instances)| instances.frame() == frame && instances.has_loaded())
    }

    /// Draws the frame. `background` draws the sky pass first, for a
    /// sky or for fog filling in behind everything without one.
    pub(crate) fn draw(&mut self, render_pass: &mut RenderPass, sky: Option<&Sky>, background: bool) {
        let frame = Window::render_frame();
        let translucent = self.transparent.frame() == frame && self.transparent.has_loaded();
        let no_nodes = self.loaded().next().is_none() && !translucent;
        if no_nodes && !background {
            self.transparent_keys.clear();
            return;
        }

        let view_bind = self.view_bind(sky.unwrap_or(&self.black_sky));
        render_pass.set_bind_group(0, &view_bind, &[]);

        if background {
            render_pass.set_pipeline(&self.sky);
            render_pass.draw(0..3, 0..1);
        }

        if no_nodes {
            self.transparent_keys.clear();
            return;
        }

        let lights_bind = storage_bind("mesh_lights_bind", &self.lights_layout, &self.lights);
        render_pass.set_bind_group(2, &lights_bind, &[]);

        for (key, instances) in self.loaded() {
            let instances_bind = self.instances_bind(instances);
            let textures_bind = textures_bind(&self.textures_layout, key, &self.white, &self.flat_normal);

            render_pass.set_bind_group(1, &instances_bind, &[]);
            render_pass.set_bind_group(3, &textures_bind, &[]);
            set_mesh(render_pass, &key.mesh, &self.opaque, &self.opaque_skinned);
            render_pass.set_vertex_buffer(1, instances.slice());

            // Base vertex stays zero, an A7 draws nothing otherwise.
            render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..instances.len());
        }

        if !translucent {
            self.transparent_keys.clear();
            return;
        }

        let instances_bind = self.instances_bind(&self.transparent);
        render_pass.set_bind_group(1, &instances_bind, &[]);

        let range = self.transparent.range().clone();
        let stride = MeshInstance::VERTEX_LAYOUT.array_stride;

        for (i, key) in self.transparent_keys.drain(..).enumerate() {
            let textures_bind = textures_bind(&self.textures_layout, &key, &self.white, &self.flat_normal);
            render_pass.set_bind_group(3, &textures_bind, &[]);
            set_mesh(
                render_pass,
                &key.mesh,
                &self.translucent,
                &self.translucent_skinned,
            );
            // The draw starts at instance zero of a slice that begins at
            // this node, its `index` attribute still names the real slot.
            let start = range.start + u64::try_from(i).expect("node count fits u64") * stride;
            render_pass.set_vertex_buffer(1, self.transparent.buffer().slice(start..range.end));
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
                BindGroupEntry {
                    binding:  3,
                    resource: BindingResource::TextureView(self.shadow.map()),
                },
            ],
        })
    }

    /// The part of the instance buffer this frame's flush landed in, so
    /// the fragment stage indexes the same elements the draw uses, and
    /// the frame's joints for the vertex stage.
    fn instances_bind(&self, instances: &VecBuffer<MeshInstance>) -> BindGroup {
        Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("mesh_instances_bind"),
            layout:  &self.instances_layout,
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::Buffer(loaded_range(instances)),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Buffer(loaded_range(&self.joints)),
                },
            ],
        })
    }
}

/// Picks the pipeline for the mesh and sets its vertex and index
/// buffers, the skin buffer too when it has one.
pub(super) fn set_mesh(pass: &mut RenderPass, mesh: &Mesh, plain: &RenderPipeline, skinned: &RenderPipeline) {
    match &mesh.skin_buffer {
        Some(skin) => {
            pass.set_pipeline(skinned);
            pass.set_vertex_buffer(2, skin.slice(..));
        }
        None => pass.set_pipeline(plain),
    }
    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    pass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint16);
}

/// A mesh pipeline and its skinned twin, the same shader from its two
/// vertex entries.
fn mesh_pipelines(
    device: &wgpu::Device,
    name: &str,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    translucent: bool,
) -> [RenderPipeline; 2] {
    [
        device.mesh_pipeline(
            &format!("{name}_pipeline"),
            layout,
            shader,
            STATIC_LAYOUTS,
            "v_main",
            translucent,
        ),
        device.mesh_pipeline(
            &format!("{name}_skinned_pipeline"),
            layout,
            shader,
            SKINNED_LAYOUTS,
            "v_skinned",
            translucent,
        ),
    ]
}

fn shader(device: &wgpu::Device, label: &str, source: String) -> wgpu::ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label:  label.into(),
        source: ShaderSource::Wgsl(source.into()),
    })
}

/// The frame's view, the sky cube and the sun's shadow map, one layer
/// per cascade.
fn view_layout(device: &wgpu::Device) -> BindGroupLayout {
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

/// The instances for the fragment stage and the joints for the vertex
/// stage. Four bind groups is every lane's limit, so the two share one.
fn instances_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "mesh_instances_layout".into(),
        entries: &[
            storage_entry(0, ShaderStages::FRAGMENT),
            storage_entry(1, ShaderStages::VERTEX),
        ],
    })
}

/// The base color texture and the normal map of a draw.
fn textures_layout(device: &wgpu::Device) -> BindGroupLayout {
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

fn storage_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
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

/// The part of a loaded `VecBuffer` that this frame's flush landed in.
fn loaded_range<T>(buffer: &VecBuffer<T>) -> BufferBinding<'_> {
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
