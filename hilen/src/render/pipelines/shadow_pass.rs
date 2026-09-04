use std::array::from_fn;

use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferBindingType, BufferUsages, CommandEncoder, Extent3d, LoadOp, Operations, PipelineLayoutDescriptor,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, ShaderModule, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::{
    gm::volume::Mat4,
    render::{
        SHADOW_CASCADES,
        bind_cache::{CachedBind, StorageKey, cached},
        buffer_helper::BufferHelper,
        data::MeshInstance,
        device_helper::{DeviceHelper, SHADOW_MAP_FORMAT},
        pipelines::mesh_pipeline::{MeshKey, SKINNED_LAYOUTS, STATIC_LAYOUTS, set_mesh, storage_bind},
        uniform::make_storage_layout,
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
    plain:         RenderPipeline,
    skinned:       RenderPipeline,
    joints_layout: BindGroupLayout,
    /// The bind over the frame's joints, held while the buffer stays.
    joints_bind:   Option<CachedBind<StorageKey>>,
    cascades:      [Cascade; SHADOW_CASCADES],
    /// Every layer at once, what the mesh shader reads.
    map:           TextureView,
    /// Texels along each side of every layer.
    map_size:      u32,
}

struct Cascade {
    view_proj: Buffer,
    bind:      BindGroup,
    layer:     TextureView,
}

impl ShadowPass {
    pub(crate) fn new(device: &wgpu::Device, shader: &ShaderModule, map_size: u32) -> Self {
        let view_layout = view_layout(device);
        let joints_layout = make_storage_layout("shadow_joints_layout", ShaderStages::VERTEX);
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "shadow_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout), Some(&joints_layout)],
            immediate_size:     0,
        });

        let map = shadow_map(device, map_size);
        let cascades = from_fn(|index| {
            let view_proj = device.buffer(&Mat4::IDENTITY, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
            Cascade {
                bind: device.bind(&view_proj, &view_layout),
                view_proj,
                layer: layer_view(&map, index),
            }
        });

        Self {
            plain: device.shadow_pipeline("shadow_pipeline", &layout, shader, STATIC_LAYOUTS, "v_main"),
            skinned: device.shadow_pipeline(
                "shadow_skinned_pipeline",
                &layout,
                shader,
                SKINNED_LAYOUTS,
                "v_skinned",
            ),
            joints_layout,
            joints_bind: None,
            cascades,
            map: map_view(&map),
            map_size,
        }
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
    /// frame and `joints` the joint matrices they point at.
    pub(crate) fn draw<'a>(
        &mut self,
        encoder: &mut CommandEncoder,
        view_projs: &[Mat4; SHADOW_CASCADES],
        batches: &[(&'a MeshKey, &'a VecBuffer<MeshInstance>)],
        joints: &VecBuffer<Mat4>,
    ) {
        let joints_bind = cached(&mut self.joints_bind, StorageKey::of(joints), || {
            storage_bind("shadow_joints_bind", &self.joints_layout, joints)
        });

        for (cascade, view_proj) in self.cascades.iter().zip(view_projs) {
            cascade.view_proj.update(*view_proj);

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label:                    "shadow_pass".into(),
                color_attachments:        &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view:        &cascade.layer,
                    depth_ops:   Some(Operations {
                        load:  LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set:      None,
                timestamp_writes:         None,
                multiview_mask:           None,
            });

            pass.set_bind_group(0, &cascade.bind, &[]);
            pass.set_bind_group(1, joints_bind, &[]);

            for (key, instances) in batches {
                set_mesh(&mut pass, &key.mesh, &self.plain, &self.skinned);
                pass.set_vertex_buffer(1, instances.slice());
                pass.draw_indexed(0..key.mesh.index_count, 0, 0..instances.len());
            }
        }
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
