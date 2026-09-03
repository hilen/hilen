use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BlendState, Buffer, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, FrontFace,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, StencilFaceState, StencilOperation, StencilState,
    VertexBufferLayout, VertexState,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    render::to_bytes::ToBytes,
    window::{BufferUsages, PolygonMode, image::Texture, msaa_sample_count, surface_texture_format},
};

/// The stencil test of everything drawn into the frame. The stencil
/// holds the rounded clip mask as a nesting depth, so a fragment draws
/// only where the stencil equals the pass reference, which the UI
/// drawer keeps at the depth of the clip it is inside. With no clip
/// active the buffer is all zero, the reference is zero and every
/// fragment passes.
const CLIP_TEST: StencilFaceState = StencilFaceState {
    compare:       CompareFunction::Equal,
    fail_op:       StencilOperation::Keep,
    depth_fail_op: StencilOperation::Keep,
    pass_op:       StencilOperation::Keep,
};

/// The shadow map's format, a plain depth texture every lane can read
/// by texel.
#[cfg(feature = "scene")]
pub(crate) const SHADOW_MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub(crate) fn depth_stencil_state() -> DepthStencilState {
    DepthStencilState {
        format:              Texture::DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare:       Some(CompareFunction::Less),
        stencil:             StencilState {
            front:      CLIP_TEST,
            back:       CLIP_TEST,
            read_mask:  !0,
            write_mask: 0,
        },
        bias:                DepthBiasState::default(),
    }
}

pub(crate) trait DeviceHelper {
    fn buffer<T: ToBytes + ?Sized>(&self, data: &T, usage: BufferUsages) -> Buffer;

    fn buffer_from_bytes(&self, data: &[u8], usage: BufferUsages) -> Buffer;

    fn bind(&self, buffer: &Buffer, layout: &BindGroupLayout) -> BindGroup;

    /// Every pipeline passes `Less`, so at equal z the first draw wins,
    /// except the path pipeline, which passes `LessEqual` for painter
    /// order within one `DrawingView`. See `UIPathPipeline`.
    fn pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        depth_compare: CompareFunction,
        topology: PrimitiveTopology,
        vertex_layout: &'static [VertexBufferLayout],
    ) -> RenderPipeline;

    /// A pipeline that writes only the stencil. Where its fragment
    /// survives and the stencil already equals the reference, `op`
    /// moves the stencil one step, so a rounded clip enters by
    /// incrementing inside its shape and leaves by decrementing the
    /// same shape. No color and no depth is written.
    fn mask_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        vertex_layout: &'static [VertexBufferLayout],
        op: StencilOperation,
    ) -> RenderPipeline;

    /// A pipeline for solid geometry. Back faces are culled, the fragment
    /// replaces the target since a scene is opaque, depth and the stencil
    /// clip test are the frame's shared ones.
    /// A translucent one blends over what is drawn and leaves the
    /// depth alone, so the nodes behind it stay visible through it.
    #[cfg(feature = "scene")]
    fn mesh_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        vertex_layout: &'static [VertexBufferLayout],
        transparent: bool,
    ) -> RenderPipeline;

    /// The skybox, one triangle over the viewport with no depth test
    /// and no depth write, drawn before the nodes cover it.
    #[cfg(feature = "scene")]
    fn sky_pipeline(&self, label: &str, layout: &PipelineLayout, shader: &ShaderModule) -> RenderPipeline;

    /// The sun's depth pass into the shadow map, no color target and a
    /// slope scaled bias against acne.
    #[cfg(feature = "scene")]
    fn shadow_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
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
        depth_compare: CompareFunction,
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
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil:  DepthStencilState {
                depth_compare: Some(depth_compare),
                ..depth_stencil_state()
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

    fn mask_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        vertex_layout: &'static [VertexBufferLayout],
        op: StencilOperation,
    ) -> RenderPipeline {
        let buffers: Vec<Option<VertexBufferLayout>> = vertex_layout.iter().cloned().map(Some).collect();
        let write = StencilFaceState {
            compare:       CompareFunction::Equal,
            fail_op:       StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op:       op,
        };
        let samples = msaa_sample_count();
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
                    blend:      None,
                    write_mask: ColorWrites::empty(),
                }
                .into()],
            }
            .into(),
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          None,
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil:  DepthStencilState {
                format:              Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare:       Some(CompareFunction::Always),
                stencil:             StencilState {
                    front:      write,
                    back:       write,
                    read_mask:  !0,
                    write_mask: !0,
                },
                bias:                DepthBiasState::default(),
            }
            .into(),
            // The shader puts the shape's edge coverage in alpha, and
            // alpha to coverage turns it into a per sample mask, so the
            // clip edge is anti aliased the same way the geometry the
            // frame multisamples is. The spec forbids it on a single
            // sample, there the shader keeps the covered half instead.
            multisample:    MultisampleState {
                count:                     samples,
                mask:                      !0,
                alpha_to_coverage_enabled: samples > 1,
            },
            cache:          None,
            multiview_mask: None,
        })
    }

    #[cfg(feature = "scene")]
    fn mesh_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        vertex_layout: &'static [VertexBufferLayout],
        transparent: bool,
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
                    blend:      Some(if transparent {
                        BlendState::ALPHA_BLENDING
                    } else {
                        BlendState::REPLACE
                    }),
                    write_mask: ColorWrites::ALL,
                }
                .into()],
            }
            .into(),
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          Some(wgpu::Face::Back),
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil:  DepthStencilState {
                depth_write_enabled: Some(!transparent),
                ..depth_stencil_state()
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

    #[cfg(feature = "scene")]
    fn shadow_pipeline(
        &self,
        label: &str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
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
            fragment:       None,
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          Some(wgpu::Face::Back),
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil:  DepthStencilState {
                format:              SHADOW_MAP_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare:       Some(CompareFunction::Less),
                stencil:             StencilState::default(),
                bias:                DepthBiasState {
                    constant:    4,
                    slope_scale: 2.0,
                    clamp:       0.0,
                },
            }
            .into(),
            multisample:    MultisampleState {
                count:                     1,
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            },
            cache:          None,
            multiview_mask: None,
        })
    }

    #[cfg(feature = "scene")]
    fn sky_pipeline(&self, label: &str, layout: &PipelineLayout, shader: &ShaderModule) -> RenderPipeline {
        self.create_render_pipeline(&RenderPipelineDescriptor {
            label:          label.into(),
            layout:         layout.into(),
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
                targets:             &[ColorTargetState {
                    format:     surface_texture_format(),
                    blend:      BlendState::REPLACE.into(),
                    write_mask: ColorWrites::ALL,
                }
                .into()],
            }
            .into(),
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          None,
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil:  DepthStencilState {
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Always),
                ..depth_stencil_state()
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
