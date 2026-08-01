use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BlendState, Buffer, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, FrontFace,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, StencilState, TextureFormat, VertexBufferLayout,
    VertexState,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    render::to_bytes::ToBytes,
    window::{BufferUsages, PolygonMode, msaa_sample_count, surface_texture_format},
};

pub(crate) trait DeviceHelper {
    fn buffer<T: ToBytes + ?Sized>(&self, data: &T, usage: BufferUsages) -> Buffer;

    fn buffer_from_bytes(&self, data: &[u8], usage: BufferUsages) -> Buffer;

    fn bind(&self, buffer: &Buffer, layout: &BindGroupLayout) -> BindGroup;

    fn pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        polygon_mode: PolygonMode,
        topology: PrimitiveTopology,
        vertex_layout: &'static [VertexBufferLayout],
    ) -> RenderPipeline;
}

impl DeviceHelper for Device {
    fn buffer<T: ToBytes + ?Sized>(&self, data: &T, usage: BufferUsages) -> Buffer {
        self.buffer_from_bytes(data.to_bytes(), usage)
    }

    fn buffer_from_bytes(&self, data: &[u8], usage: BufferUsages) -> Buffer {
        self.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: data,
            usage,
        })
    }

    fn bind(&self, buffer: &Buffer, layout: &BindGroupLayout) -> BindGroup {
        self.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout,
            entries: &[BindGroupEntry {
                binding:  0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        polygon_mode: PolygonMode,
        topology: PrimitiveTopology,
        vertex_layout: &'static [VertexBufferLayout],
    ) -> RenderPipeline {
        let buffers: Vec<Option<VertexBufferLayout>> = vertex_layout.iter().cloned().map(Some).collect();
        self.create_render_pipeline(&RenderPipelineDescriptor {
            label:          label.into(),
            layout:         layout.into(),
            vertex:         VertexState {
                module:              shader,
                entry_point:         "v_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                buffers:             &buffers,
            },
            fragment:       FragmentState {
                module:              shader,
                entry_point:         "f_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                targets:             &[ColorTargetState {
                    format:     surface_texture_format(),
                    blend:      BlendState::ALPHA_BLENDING.into(),
                    write_mask: ColorWrites::ALL,
                }
                .into()],
            }
            .into(),
            primitive:      PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                // cull_mode: wgpu::Face::Back.into(),
                cull_mode: None,
                polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil:  DepthStencilState {
                format:              TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare:       Some(CompareFunction::Less),
                stencil:             StencilState::default(),
                bias:                DepthBiasState::default(),
            }
            .into(),
            multisample:    MultisampleState {
                count:                     msaa_sample_count(),
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            },
            cache:          None,
            multiview_mask: None,
        })
    }
}
