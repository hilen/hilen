//! NV12 planes to an RGBA image on the GPU. Two textures take the decoder's
//! planes as they are, one fullscreen pass converts them into the image the
//! `ImageView` draws, so aspect modes, corner radii and flips work on a video
//! frame like on any picture.

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, Extent3d, FragmentState, LoadOp, MultisampleState, Operations, Origin3d,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    VertexState,
};

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::flat::Size,
    render::uniform::{UniformBind, make_uniform_layout},
    video::decoder::VideoFrame,
    window::{Window, image::Image},
};

const SHADER: &str = include_str!("nv12.wgsl");

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Pod, Zeroable)]
struct Nv12Params {
    full_range: u32,
    bt601:      u32,
    padding:    [u32; 2],
}

struct Nv12Pipeline {
    pipeline: RenderPipeline,
    planes:   BindGroupLayout,
    params:   BindGroupLayout,
    sampler:  Sampler,
}

fn pipeline() -> &'static Nv12Pipeline {
    static PIPELINE: MainLock<Nv12Pipeline> = MainLock::new();
    PIPELINE.get_or_init(Nv12Pipeline::new)
}

impl Nv12Pipeline {
    fn new() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("nv12.wgsl"),
            source: ShaderSource::Wgsl(SHADER.into()),
        });

        let plane = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled:   false,
                view_dimension: TextureViewDimension::D2,
                sample_type:    TextureSampleType::Float { filterable: true },
            },
            count: None,
        };
        let planes = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label:   Some("video_planes_layout"),
            entries: &[
                plane(0),
                plane(1),
                BindGroupLayoutEntry {
                    binding:    2,
                    visibility: ShaderStages::FRAGMENT,
                    ty:         BindingType::Sampler(SamplerBindingType::Filtering),
                    count:      None,
                },
            ],
        });
        let params = make_uniform_layout("video_params_layout", ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("video_nv12_pipeline_layout"),
            bind_group_layouts: &[Some(&planes), Some(&params)],
            immediate_size:     0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label:          Some("video_nv12_pipeline"),
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
                    format:     TextureFormat::Rgba8Unorm,
                    blend:      Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive:      PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..PrimitiveState::default()
            },
            depth_stencil:  None,
            multisample:    MultisampleState::default(),
            cache:          None,
            multiview_mask: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("video_planes_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        Self {
            pipeline,
            planes,
            params,
            sampler,
        }
    }
}

/// The planes and the image of one video, sized to its frames.
pub(crate) struct Nv12Target {
    size:   Size<u32>,
    y:      Texture,
    uv:     Texture,
    bind:   BindGroup,
    params: UniformBind<Nv12Params>,
    image:  Weak<Image>,
}

impl Nv12Target {
    pub(crate) fn new(key: &str, size: Size<u32>) -> Self {
        let device = Window::device();
        let shared = pipeline();

        let y = plane_texture("video_y", size, TextureFormat::R8Unorm);
        let uv = plane_texture("video_uv", chroma_size(size), TextureFormat::Rg8Unorm);
        let y_view = y.create_view(&TextureViewDescriptor::default());
        let uv_view = uv.create_view(&TextureViewDescriptor::default());

        let bind = device.create_bind_group(&BindGroupDescriptor {
            label:   Some("video_planes_bind"),
            layout:  &shared.planes,
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(&y_view),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::TextureView(&uv_view),
                },
                BindGroupEntry {
                    binding:  2,
                    resource: BindingResource::Sampler(&shared.sampler),
                },
            ],
        });

        let params = UniformBind::from(shared.params.clone());
        let image = Image::render_target(&format!("{key}-{}x{}", size.width, size.height), size);

        Self {
            size,
            y,
            uv,
            bind,
            params,
            image,
        }
    }

    pub(crate) fn size(&self) -> Size<u32> {
        self.size
    }

    pub(crate) fn image(&self) -> Weak<Image> {
        self.image
    }

    /// Uploads the planes and converts them into the image.
    pub(crate) fn show(&self, frame: &VideoFrame) {
        let queue = Window::queue();
        write_plane(queue, &self.y, &frame.y, frame.y_stride, self.size);
        write_plane(
            queue,
            &self.uv,
            &frame.uv,
            frame.uv_stride,
            chroma_size(self.size),
        );
        self.params.update(Nv12Params {
            full_range: u32::from(frame.full_range),
            bt601:      u32::from(frame.bt601),
            padding:    [0; 2],
        });

        let mut encoder = Window::device().create_command_encoder(&CommandEncoderDescriptor {
            label: Some("video_convert"),
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label:                    Some("video_nv12_pass"),
                color_attachments:        &[Some(RenderPassColorAttachment {
                    view:           self.image.view(),
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
            pass.set_pipeline(&pipeline().pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_bind_group(1, self.params.bind(), &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

fn chroma_size(size: Size<u32>) -> Size<u32> {
    Size::new(size.width.div_ceil(2), size.height.div_ceil(2))
}

fn plane_texture(label: &str, size: Size<u32>, format: TextureFormat) -> Texture {
    Window::device().create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// The rows as the decoder laid them out, `stride` bytes each, so no repack.
fn write_plane(queue: &Queue, texture: &Texture, data: &[u8], stride: u32, size: Size<u32>) {
    queue.write_texture(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        data,
        TexelCopyBufferLayout {
            offset:         0,
            bytes_per_row:  Some(stride),
            rows_per_image: Some(size.height),
        },
        Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        },
    );
}
