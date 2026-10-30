use std::{array::from_fn, ops::Range};

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferBindingType, BufferUsages, CommandEncoder, Extent3d, LoadOp, Operations, PipelineLayout,
    PipelineLayoutDescriptor, RenderPass, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, ShaderModule, ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::{
    gm::volume::{Mat4, Vec4},
    render::{
        SHADOW_CASCADES,
        buffer_helper::BufferHelper,
        data::MeshInstance,
        device_helper::{DeviceHelper, SHADOW_MAP_FORMAT},
        pipelines::mesh_pipeline::{MeshKey, SKINNED_LAYOUTS, STATIC_LAYOUTS, set_mesh},
        vec_buffer::VecBuffer,
    },
};

/// The sun's depth passes, one per cascade. Each draws every opaque
/// batch of the frame from the light into its own layer of the shadow
/// map, depth only, with the vertex and instance buffers of the main
/// pass. Every cascade has its own matrix buffer and bind group, since
/// every `write_buffer` of a frame lands before its first pass and one
/// buffer would hold only the last cascade.
pub(crate) struct ShadowPass {
    /// A pipeline and its skinned twin, depth only with no fragment
    /// stage.
    plain:         [RenderPipeline; 2],
    /// The same with the stage that drops what a cutout drops, so it
    /// binds the batch's textures too.
    cutout:        [RenderPipeline; 2],
    /// The cutout of a mesh whose colors fade, made at the first such
    /// mesh from the layout and the shader kept here.
    smooth_cutout: Option<RenderPipeline>,
    cutout_layout: PipelineLayout,
    shader:        ShaderModule,
    cascades:      [Cascade; SHADOW_CASCADES],
    /// Every layer at once, what the mesh shader reads.
    map:           TextureView,
    /// Texels along each side of every layer.
    map_size:      u32,
}

/// What a cascade's pass reads, see `ShadowView` in shadow.wgsl: the view
/// projection of a point relative to `origin`, the camera.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShadowView {
    view_proj: Mat4,
    origin:    Vec4,
}

struct Cascade {
    view_proj: Buffer,
    bind:      BindGroup,
    layer:     TextureView,
}

/// One opaque draw of the frame: a mesh, its range of the instances and,
/// for a cut out batch, the bind over its textures.
pub(crate) struct ShadowBatch<'a> {
    pub key:      &'a MeshKey,
    /// The instances each cascade draws, only the ones in its box.
    pub ranges:   [Range<u32>; SHADOW_CASCADES],
    pub textures: Option<BindGroup>,
}

impl ShadowPass {
    /// `instances_layout` and `textures_layout` are the mesh pipeline's
    /// own, the passes draw with the binds it makes.
    pub(crate) fn new(
        device: &wgpu::Device,
        shader: &ShaderModule,
        map_size: u32,
        instances_layout: &BindGroupLayout,
        textures_layout: &BindGroupLayout,
    ) -> Self {
        let view_layout = view_layout(device);
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "shadow_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout), Some(instances_layout)],
            immediate_size:     0,
        });
        let cutout_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "shadow_cutout_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout), Some(instances_layout), Some(textures_layout)],
            immediate_size:     0,
        });

        let map = shadow_map(device, map_size);
        let cascades = from_fn(|index| {
            let view = ShadowView {
                view_proj: Mat4::IDENTITY,
                origin:    Vec4::ZERO,
            };
            let view_proj = device.buffer(&view, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
            Cascade {
                bind: device.bind(&view_proj, &view_layout),
                view_proj,
                layer: layer_view(&map, index),
            }
        });

        Self {
            plain: [
                device.shadow_pipeline("shadow_pipeline", &layout, shader, STATIC_LAYOUTS, "v_main", None),
                device.shadow_pipeline(
                    "shadow_skinned_pipeline",
                    &layout,
                    shader,
                    SKINNED_LAYOUTS,
                    "v_skinned",
                    None,
                ),
            ],
            cutout: [
                device.shadow_pipeline(
                    "shadow_cutout_pipeline",
                    &cutout_layout,
                    shader,
                    STATIC_LAYOUTS,
                    "v_cutout",
                    Some("f_cutout"),
                ),
                device.shadow_pipeline(
                    "shadow_cutout_skinned_pipeline",
                    &cutout_layout,
                    shader,
                    SKINNED_LAYOUTS,
                    "v_cutout_skinned",
                    Some("f_cutout"),
                ),
            ],
            cascades,
            smooth_cutout: None,
            cutout_layout,
            shader: shader.clone(),
            map: map_view(&map),
            map_size,
        }
    }

    /// Makes the cutout pipeline of a mesh whose colors fade.
    pub(crate) fn make_smooth(&mut self, device: &wgpu::Device) {
        self.smooth_cutout = Some(device.shadow_pipeline(
            "shadow_smooth_cutout_pipeline",
            &self.cutout_layout,
            &self.shader,
            STATIC_LAYOUTS,
            "v_smooth_cutout",
            Some("f_smooth_cutout"),
        ));
    }

    pub(crate) fn map(&self) -> &TextureView {
        &self.map
    }

    /// Remakes the maps at `map_size` texels a side when that changed.
    pub(crate) fn fit(&mut self, device: &wgpu::Device, map_size: u32) {
        if map_size == self.map_size {
            return;
        }
        let map = shadow_map(device, map_size);
        for (index, cascade) in self.cascades.iter_mut().enumerate() {
            cascade.layer = layer_view(&map, index);
        }
        self.map = map_view(&map);
        self.map_size = map_size;
    }

    /// Draws every cascade's layer, `batches` the opaque draws of the
    /// frame and `instances_bind` the mesh pipeline's bind over
    /// `instances` and the joints they point at. The view projections
    /// take a point relative to `origin`, the camera.
    pub(crate) fn draw(
        &self,
        encoder: &mut CommandEncoder,
        view_projs: &[Mat4; SHADOW_CASCADES],
        origin: Vec4,
        instances: &VecBuffer<MeshInstance>,
        batches: &[ShadowBatch],
        instances_bind: &BindGroup,
    ) {
        for (index, (cascade, view_proj)) in self.cascades.iter().zip(view_projs).enumerate() {
            cascade.view_proj.update(ShadowView {
                view_proj: *view_proj,
                origin,
            });

            let mut pass = cascade.pass(encoder);
            pass.set_bind_group(0, &cascade.bind, &[]);
            pass.set_bind_group(1, instances_bind, &[]);

            for batch in batches {
                let range = &batch.ranges[index];
                if range.is_empty() {
                    continue;
                }
                let pipelines = match &batch.textures {
                    Some(textures) => {
                        pass.set_bind_group(2, textures, &[]);
                        &self.cutout
                    }
                    None => &self.plain,
                };
                set_mesh(&mut pass, &batch.key.mesh, pipelines);
                // A plain batch writes only the depth of its position, the
                // plain pipeline draws a smooth mesh the same.
                if batch.textures.is_some()
                    && batch.key.mesh.smooth_colors
                    && let Some(smooth) = &self.smooth_cutout
                {
                    pass.set_pipeline(smooth);
                }
                pass.set_vertex_buffer(1, instances.elements(range.clone()));
                pass.draw_indexed(0..batch.key.mesh.index_count, 0, 0..range.end - range.start);
            }
        }
    }

    /// Empties every layer, for a frame with nothing opaque to cast.
    pub(crate) fn clear(&self, encoder: &mut CommandEncoder) {
        for cascade in &self.cascades {
            cascade.pass(encoder);
        }
    }
}

impl Cascade {
    /// The pass that draws this cascade's layer, cleared to the far end.
    fn pass<'a>(&self, encoder: &'a mut CommandEncoder) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label:                    "shadow_pass".into(),
            color_attachments:        &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view:        &self.layer,
                depth_ops:   Some(Operations {
                    load:  LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set:      None,
            timestamp_writes:         None,
            multiview_mask:           None,
        })
    }
}

/// One cascade's matrix alone, what its pass binds.
fn view_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   "shadow_view_layout".into(),
        entries: &[BindGroupLayoutEntry {
            binding:    0,
            visibility: ShaderStages::VERTEX,
            ty:         BindingType::Buffer {
                ty:                 BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size:   None,
            },
            count:      None,
        }],
    })
}

fn layer_view(map: &Texture, index: usize) -> TextureView {
    map.create_view(&TextureViewDescriptor {
        label: Some("shadow_map_layer"),
        dimension: Some(TextureViewDimension::D2),
        base_array_layer: u32::try_from(index).expect("a handful of cascades"),
        array_layer_count: Some(1),
        ..TextureViewDescriptor::default()
    })
}

fn map_view(map: &Texture) -> TextureView {
    map.create_view(&TextureViewDescriptor {
        label: Some("shadow_map"),
        dimension: Some(TextureViewDimension::D2Array),
        ..TextureViewDescriptor::default()
    })
}

/// One layer per cascade.
fn shadow_map(device: &wgpu::Device, map_size: u32) -> Texture {
    device.create_texture(&TextureDescriptor {
        label:           "shadow_map".into(),
        size:            Extent3d {
            width:                 map_size,
            height:                map_size,
            depth_or_array_layers: u32::try_from(SHADOW_CASCADES).expect("a handful of cascades"),
        },
        mip_level_count: 1,
        sample_count:    1,
        dimension:       TextureDimension::D2,
        format:          SHADOW_MAP_FORMAT,
        usage:           TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats:    &[],
    })
}
