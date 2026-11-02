//! The sky view table of a sky with air, see `SKY_VIEW_SIZE` in
//! atmosphere.wgsl: the light of the air and what gets through it in every
//! direction from the camera, built once a frame before the frame's pass,
//! which reads it for every sky pixel.

use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, Color as WgpuColor,
    ColorTargetState, ColorWrites, CommandEncoder, Extent3d, FilterMode, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerDescriptor, ShaderModule, ShaderStages, StoreOp, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

use super::{
    air_textures::AirTextures,
    mesh_binds::{sampler_entry, texture_entry},
};
use crate::{
    render::{
        SceneUniform,
        bind_cache::{CachedBind, cached},
    },
    window::Window,
};

/// `SKY_VIEW_SIZE` of atmosphere.wgsl.
const WIDTH: u32 = 512;
const HEIGHT: u32 = 256;
const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub(crate) struct SkyView {
    layout:             BindGroupLayout,
    pipeline:           RenderPipeline,
    /// The air's light, then what gets through it.
    pub(crate) light:   TextureView,
    pub(crate) through: TextureView,
    /// Repeats across the width, where the turn goes on past a circle.
    pub(crate) sampler: Sampler,
    /// Clamps, for the air's tables this pass reads.
    table_sampler:      Sampler,
    bind:               Option<CachedBind<(TextureView, TextureView)>>,
}

impl SkyView {
    /// `shader` holds the common part, the air and `sky_view.wgsl`.
    pub(crate) fn new(device: &wgpu::Device, shader: &ShaderModule) -> Self {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label:   "sky_view_layout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding:    0,
                    visibility: ShaderStages::FRAGMENT,
                    ty:         BindingType::Buffer {
                        ty:                 BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    },
                    count:      None,
                },
                sampler_entry(2),
                texture_entry(4, TextureViewDimension::D2),
                texture_entry(5, TextureViewDimension::D2),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "sky_view_pipeline_layout".into(),
            bind_group_layouts: &[Some(&layout)],
            immediate_size:     0,
        });
        let target = Some(ColorTargetState {
            format:     FORMAT,
            blend:      None,
            write_mask: ColorWrites::ALL,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label:          "sky_view_pipeline".into(),
            layout:         Some(&pipeline_layout),
            vertex:         VertexState {
                module:              shader,
                entry_point:         "v_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                buffers:             &[],
            },
            fragment:       FragmentState {
                module:              shader,
                entry_point:         "f_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                targets:             &[target.clone(), target],
            }
            .into(),
            primitive:      PrimitiveState::default(),
            depth_stencil:  None,
            multisample:    MultisampleState::default(),
            cache:          None,
            multiview_mask: None,
        });
        let table = |label: &str| {
            device
                .create_texture(&TextureDescriptor {
                    label:           label.into(),
                    size:            Extent3d {
                        width:                 WIDTH,
                        height:                HEIGHT,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count:    1,
                    dimension:       TextureDimension::D2,
                    format:          FORMAT,
                    usage:           TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats:    &[],
                })
                .create_view(&TextureViewDescriptor::default())
        };
        let linear = |address_mode_u: AddressMode, label: &str| {
            device.create_sampler(&SamplerDescriptor {
                label: label.into(),
                address_mode_u,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            })
        };
        Self {
            layout,
            pipeline,
            light: table("sky_view_light"),
            through: table("sky_view_through"),
            sampler: linear(AddressMode::Repeat, "sky_view_sampler"),
            table_sampler: linear(AddressMode::ClampToEdge, "sky_view_table_sampler"),
            bind: None,
        }
    }

    /// Builds the table for `view`, loaded into `view_buffer`, before the
    /// pass that reads it opens. Only a sky with air reads it, see
    /// `space_sky`.
    pub(crate) fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &SceneUniform,
        view_buffer: &Buffer,
        air: &AirTextures,
    ) {
        if view.space.x < 0.5 || view.air[5].w < 0.5 {
            return;
        }
        let bind = cached(
            &mut self.bind,
            (air.transmittance.clone(), air.multi.clone()),
            || {
                Window::device().create_bind_group(&BindGroupDescriptor {
                    label:   Some("sky_view_bind"),
                    layout:  &self.layout,
                    entries: &[
                        BindGroupEntry {
                            binding:  0,
                            resource: view_buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding:  2,
                            resource: BindingResource::Sampler(&self.table_sampler),
                        },
                        BindGroupEntry {
                            binding:  4,
                            resource: BindingResource::TextureView(&air.transmittance),
                        },
                        BindGroupEntry {
                            binding:  5,
                            resource: BindingResource::TextureView(&air.multi),
                        },
                    ],
                })
            },
        );
        let attachment = |view| {
            Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load:  LoadOp::Clear(WgpuColor::BLACK),
                    store: StoreOp::Store,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label:                    "sky_view_pass".into(),
            color_attachments:        &[attachment(&self.light), attachment(&self.through)],
            depth_stencil_attachment: None,
            occlusion_query_set:      None,
            timestamp_writes:         None,
            multiview_mask:           None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
