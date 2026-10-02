use bytemuck::{Pod, Zeroable};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Extent3d, FilterMode,
    FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureDescriptor,
    TextureDimension, TextureUsages, TextureView, TextureViewDescriptor, VertexState,
};

use crate::{
    gm::flat::Size,
    render::{
        device_helper::depth_stencil_state,
        uniform::{UniformBind, make_uniform_layout},
    },
    window::{
        PassTarget, Window,
        image::{Image, Texture},
        msaa_sample_count, surface_texture_format,
    },
};

const GROUP_CODE: &str = include_str!("shaders/ui_group.wgsl");

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Zeroable, Pod)]
struct GroupParams {
    opacity:  f32,
    _padding: [f32; 3],
}

/// What one group draws into: a color image of the frame size with its own
/// depth and stencil, multisampled like the frame.
struct GroupTarget {
    size:  Size<u32>,
    /// The multisampled color, none when multisampling is off.
    msaa:  Option<TextureView>,
    color: TextureView,
    depth: TextureView,
    bind:  BindGroup,
}

/// Fades a view and its subviews as one picture, see
/// `ViewData::set_group_opacity`. The subtree draws into an image of its
/// own at full strength, then that image draws into the frame once with
/// the opacity, so parts that overlap do not show through each other.
///
/// A group inside a group draws into the next image, one per nesting
/// level. They are made on first use and have the size of the frame.
pub struct UIGroupPipeline {
    pipeline:      RenderPipeline,
    sampler:       Sampler,
    params_layout: BindGroupLayout,

    targets: Vec<GroupTarget>,

    // Queued uniform writes execute together before any pass runs, so
    // every group of a frame needs its own params. The pool is indexed
    // per group and resets when the frame changes.
    params: Vec<UniformBind<GroupParams>>,
    drawn:  usize,
    frame:  u64,
}

impl Default for UIGroupPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("ui_group.wgsl"),
            source: ShaderSource::Wgsl(GROUP_CODE.into()),
        });

        let params_layout = make_uniform_layout("ui_group_params_layout", ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("ui_group_pipeline_layout"),
            bind_group_layouts: &[Some(Image::uniform_layout()), Some(&params_layout)],
            immediate_size:     0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label:          Some("ui_group_pipeline"),
            layout:         Some(&layout),
            vertex:         VertexState {
                module:              &shader,
                entry_point:         Some("v_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers:             &[],
            },
            fragment:       Some(FragmentState {
                module:              &shader,
                entry_point:         Some("f_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets:             &[Some(ColorTargetState {
                    format:     surface_texture_format(),
                    blend:      Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          None,
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            // The group lies over everything drawn before it, like a
            // translucent view, and leaves the depth alone. The stencil
            // test keeps it inside the rounded clip it sits in.
            depth_stencil:  Some(DepthStencilState {
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Always),
                ..depth_stencil_state()
            }),
            multisample:    MultisampleState {
                count:                     msaa_sample_count(),
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            },
            cache:          None,
            multiview_mask: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("ui_group_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        Self {
            pipeline,
            sampler,
            params_layout,
            targets: vec![],
            params: vec![],
            drawn: 0,
            frame: 0,
        }
    }
}

impl UIGroupPipeline {
    /// What the group at nesting `level` draws into, for a frame of `size`.
    pub(crate) fn target(&mut self, level: usize, size: Size<u32>) -> PassTarget {
        while self.targets.len() <= level {
            let target = self.make_target(size);
            self.targets.push(target);
        }
        if self.targets[level].size != size {
            self.targets[level] = self.make_target(size);
        }
        let target = &self.targets[level];
        PassTarget {
            color:   target.msaa.clone().unwrap_or_else(|| target.color.clone()),
            resolve: target.msaa.as_ref().map(|_| target.color.clone()),
            depth:   target.depth.clone(),
        }
    }

    /// Draws the image of the group at `level` over the pass with `opacity`.
    pub(crate) fn draw(&mut self, pass: &mut RenderPass, level: usize, opacity: f32) {
        let frame = Window::render_frame();
        if self.frame != frame {
            self.frame = frame;
            self.drawn = 0;
        }
        if self.drawn == self.params.len() {
            self.params.push(UniformBind::from(self.params_layout.clone()));
        }
        let params = &self.params[self.drawn];
        self.drawn += 1;
        params.update(GroupParams {
            opacity,
            _padding: [0.0; 3],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.targets[level].bind, &[]);
        pass.set_bind_group(1, params.bind(), &[]);
        pass.draw(0..3, 0..1);
    }

    fn make_target(&self, size: Size<u32>) -> GroupTarget {
        let device = Window::device();
        let samples = msaa_sample_count();
        let color_texture = |label: &str, sample_count: u32, usage: TextureUsages| {
            device
                .create_texture(&TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width:                 size.width,
                        height:                size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: TextureDimension::D2,
                    format: surface_texture_format(),
                    usage,
                    view_formats: &[],
                })
                .create_view(&TextureViewDescriptor::default())
        };

        let color = color_texture(
            "ui_group_color",
            1,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );
        let msaa =
            (samples > 1).then(|| color_texture("ui_group_msaa", samples, TextureUsages::RENDER_ATTACHMENT));
        let depth = Texture::create_depth_texture(device, size, samples, "ui_group_depth").view;

        let bind = device.create_bind_group(&BindGroupDescriptor {
            label:   Some("ui_group_bind"),
            layout:  Image::uniform_layout(),
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(&color),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        GroupTarget {
            size,
            msaa,
            color,
            depth,
            bind,
        }
    }
}
