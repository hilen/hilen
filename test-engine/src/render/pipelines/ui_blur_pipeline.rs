use bytemuck::{Pod, Zeroable};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    BlendState, Color, ColorTargetState, ColorWrites, CommandEncoder, Extent3d, FilterMode, FragmentState,
    FrontFace, LoadOp, MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, Sampler, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::{
    gm::flat::{Point, Size},
    render::uniform::{UniformBind, make_uniform_layout},
    window::{Window, image::Image, surface_texture_format},
};

const BLUR_CODE: &str = include_str!("shaders/blur.wgsl");

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Zeroable, Pod)]
struct BlurParams {
    direction: Point,
    sigma:     f32,
    _padding:  f32,
}

struct BlurTarget {
    view: TextureView,
    bind: BindGroup,
}

struct BlurTargets {
    size:      Size<u32>,
    half:      BlurTarget,
    quarter_a: BlurTarget,
    quarter_b: BlurTarget,
}

/// Produces a blurred copy of the scene texture for backdrop blur.
/// The scene is downsampled in two steps to quarter resolution, then
/// blurred by a separable gaussian, horizontal and vertical. The
/// result stays in the quarter texture behind `output_bind`. Sigma
/// saturates around 10 at quarter resolution because the shader caps
/// its taps, so very large radii stop getting blurrier.
pub struct UIBlurPipeline {
    copy: RenderPipeline,
    blur: RenderPipeline,

    sampler:       Sampler,
    params_layout: BindGroupLayout,

    targets: Option<BlurTargets>,

    // Queued uniform writes execute together before any pass runs, so
    // every barrier in a frame needs its own params pair. The pool is
    // indexed per barrier and resets when the frame changes.
    params:  Vec<[UniformBind<BlurParams>; 2]>,
    barrier: usize,
    frame:   u64,
}

impl Default for UIBlurPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("blur.wgsl"),
            source: ShaderSource::Wgsl(BLUR_CODE.into()),
        });

        let params_layout = make_uniform_layout("blur_params_layout", ShaderStages::FRAGMENT);

        let copy_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("blur_copy_pipeline_layout"),
            bind_group_layouts: &[Some(Image::uniform_layout())],
            immediate_size:     0,
        });

        let blur_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("blur_pipeline_layout"),
            bind_group_layouts: &[Some(Image::uniform_layout()), Some(&params_layout)],
            immediate_size:     0,
        });

        let copy = effect_pipeline(device, "blur_copy_pipeline", &shader, "f_copy", &copy_layout);
        let blur = effect_pipeline(device, "blur_pipeline", &shader, "f_blur", &blur_layout);

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("blur_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        Self {
            copy,
            blur,
            sampler,
            params_layout,
            targets: None,
            params: vec![],
            barrier: 0,
            frame: 0,
        }
    }
}

impl UIBlurPipeline {
    /// Encodes the blur chain for the current scene content. The draws
    /// already recorded execute first, so the result is the blur of
    /// everything drawn before this call.
    pub(crate) fn blur(
        &mut self,
        encoder: &mut CommandEncoder,
        scene: &TextureView,
        size: Size<u32>,
        radius: f32,
    ) {
        self.ensure_targets(size);

        let sigma = (radius / 4.0).max(0.5);
        let pair = self.next_params();
        pair[0].update(BlurParams {
            direction: Point::new(1.0, 0.0),
            sigma,
            _padding: 0.0,
        });
        pair[1].update(BlurParams {
            direction: Point::new(0.0, 1.0),
            sigma,
            _padding: 0.0,
        });

        let scene_bind = self.bind_texture(scene);
        let pair = &self.params[self.barrier - 1];
        let targets = self.targets.as_ref().expect("blur targets ensured");

        effect_pass(encoder, &targets.half.view, &self.copy, &scene_bind, None);
        effect_pass(
            encoder,
            &targets.quarter_a.view,
            &self.copy,
            &targets.half.bind,
            None,
        );
        effect_pass(
            encoder,
            &targets.quarter_b.view,
            &self.blur,
            &targets.quarter_a.bind,
            Some(pair[0].bind()),
        );
        effect_pass(
            encoder,
            &targets.quarter_a.view,
            &self.blur,
            &targets.quarter_b.bind,
            Some(pair[1].bind()),
        );
    }

    /// The blurred scene of the latest `blur` call.
    pub(crate) fn output_bind(&self) -> &BindGroup {
        &self.targets.as_ref().expect("blur ran this frame").quarter_a.bind
    }

    /// Draws the scene texture onto the current pass target, used as
    /// the final copy to the surface when the frame rendered into an
    /// intermediate texture.
    pub(crate) fn present(&self, pass: &mut wgpu::RenderPass, scene: &TextureView) {
        let bind = self.bind_texture(scene);
        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }

    fn bind_texture(&self, view: &TextureView) -> BindGroup {
        Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("blur_source_bind"),
            layout:  Image::uniform_layout(),
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn next_params(&mut self) -> &[UniformBind<BlurParams>; 2] {
        let frame = Window::render_frame();
        if self.frame != frame {
            self.frame = frame;
            self.barrier = 0;
        }

        if self.barrier == self.params.len() {
            self.params.push([
                UniformBind::from(self.params_layout.clone()),
                UniformBind::from(self.params_layout.clone()),
            ]);
        }

        self.barrier += 1;
        &self.params[self.barrier - 1]
    }

    fn ensure_targets(&mut self, size: Size<u32>) {
        if self.targets.as_ref().is_some_and(|targets| targets.size == size) {
            return;
        }

        let half = Size::new((size.width / 2).max(1), (size.height / 2).max(1));
        let quarter = Size::new((size.width / 4).max(1), (size.height / 4).max(1));

        self.targets = Some(BlurTargets {
            size,
            half: self.make_target("blur_half", half),
            quarter_a: self.make_target("blur_quarter_a", quarter),
            quarter_b: self.make_target("blur_quarter_b", quarter),
        });
    }

    fn make_target(&self, label: &str, size: Size<u32>) -> BlurTarget {
        let texture = Window::device().create_texture(&TextureDescriptor {
            label:           Some(label),
            size:            Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats:    &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        let bind = self.bind_texture(&view);

        BlurTarget { view, bind }
    }
}

fn effect_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    entry: &str,
    layout: &wgpu::PipelineLayout,
) -> RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label:          label.into(),
        layout:         layout.into(),
        vertex:         wgpu::VertexState {
            module:              shader,
            entry_point:         "v_main".into(),
            compilation_options: PipelineCompilationOptions::default(),
            buffers:             &[],
        },
        fragment:       FragmentState {
            module:              shader,
            entry_point:         entry.into(),
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
        depth_stencil:  None,
        multisample:    MultisampleState {
            count:                     1,
            mask:                      !0,
            alpha_to_coverage_enabled: false,
        },
        cache:          None,
        multiview_mask: None,
    })
}

fn effect_pass(
    encoder: &mut CommandEncoder,
    target: &TextureView,
    pipeline: &RenderPipeline,
    source: &BindGroup,
    params: Option<&BindGroup>,
) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label:                    Some("blur_pass"),
        color_attachments:        &[Some(RenderPassColorAttachment {
            view:           target,
            depth_slice:    None,
            resolve_target: None,
            ops:            Operations {
                load:  LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set:      None,
        timestamp_writes:         None,
        multiview_mask:           None,
    });

    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, source, &[]);

    if let Some(params) = params {
        pass.set_bind_group(1, params, &[]);
    }

    pass.draw(0..3, 0..1);
}
