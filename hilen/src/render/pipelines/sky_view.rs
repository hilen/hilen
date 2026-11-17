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
    air_textures::{AirPlace, AirTextures},
    mesh_binds::{sampler_entry, texture_entry},
};
use crate::{
    gm::volume::Vec4,
    render::{
        SceneUniform,
        bind_cache::{CachedBind, cached},
    },
    scene::MAX_OCCLUDERS,
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
    /// What the table holds now, built again only when the view moves
    /// past it.
    built:              Option<SkyKey>,
}

/// What of a view the table depends on. It is built for a sun of 1, the
/// sky pass gives the sun its color.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SkyKey {
    place:      AirPlace,
    /// The ground radius and the shape of the air, `air[0].w` and
    /// `air[1..6]`.
    shape:      [Vec4; 6],
    occluders:  [Vec4; MAX_OCCLUDERS],
    transmit:   [Vec4; MAX_OCCLUDERS],
    /// The count of the occluders and the size of the sun's disc.
    sun_disc:   (f32, f32),
    brightness: f32,
}

/// A change of the brightness of the air under this share draws the same.
const BRIGHTNESS_STEP: f32 = 1e-3;

impl SkyKey {
    fn of(view: &SceneUniform) -> Self {
        let mut shape = view.air;
        shape[0] = Vec4::new(0.0, 0.0, 0.0, view.air[0].w);
        Self {
            place: AirPlace {
                from_center: -view.air[0].truncate(),
                to_sun:      -view.sun_dir.truncate(),
            },
            shape,
            occluders: view.occluders,
            transmit: view.transmit,
            sun_disc: (view.occlusion.x, view.occlusion.y),
            brightness: view.occlusion.w,
        }
    }

    /// Whether the table built for `self` still holds for `other`. An
    /// occluder may move by the same turn as the sun, seen from the camera.
    fn holds(&self, other: &Self) -> bool {
        let occluders_near = self.occluders.iter().zip(&other.occluders).all(|(a, b)| {
            a.w.to_bits() == b.w.to_bits()
                && a.truncate().distance(b.truncate()) <= 5e-4 * a.truncate().length().max(1.0)
        });
        self.shape == other.shape
            && self.transmit == other.transmit
            && self.sun_disc == other.sun_disc
            && (self.brightness - other.brightness).abs() <= BRIGHTNESS_STEP * self.brightness.abs().max(1e-6)
            && occluders_near
            && self.place.near(&other.place, self.shape[0].w)
    }
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
            built: None,
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
        // 512 by 256 texels of 24 steps each, for a sky that moves slowly.
        let key = SkyKey::of(view);
        if self.built.is_some_and(|built| built.holds(&key)) {
            return;
        }
        self.built = Some(key);
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::gm::volume::Vec3;

    const GROUND: f32 = 20_000.0;

    fn view(height: f32, sun: Vec3) -> SceneUniform {
        let mut view = SceneUniform::default();
        view.air[0] = Vec3::new(0.0, -(GROUND + height), 0.0).extend(GROUND);
        view.air[1] = Vec4::new(5.8e-6, 1.35e-5, 3.31e-5, GROUND + 60_000.0);
        view.sun_dir = (-sun).normalize().extend(0.0);
        view.occlusion = Vec4::new(1.0, 0.0047, 0.0, 0.8);
        view.occluders[0] = Vec4::new(0.0, -(GROUND + height), 0.0, GROUND);
        view
    }

    // The table was marched every frame, 3 million steps, for a sky that
    // turns once in 24 minutes.
    #[test]
    fn a_table_holds_while_the_sky_barely_moves() {
        let built = SkyKey::of(&view(2.0, Vec3::new(0.3, 1.0, 0.0)));
        assert!(built.holds(&SkyKey::of(&view(2.2, Vec3::new(0.3, 1.0, 0.0001)))));
    }

    #[test]
    fn a_table_is_built_again_when_the_sun_the_height_or_the_air_moves() {
        let built = SkyKey::of(&view(2.0, Vec3::new(0.3, 1.0, 0.0)));
        assert!(
            !built.holds(&SkyKey::of(&view(2.0, Vec3::new(0.3, 1.0, 0.01)))),
            "the sun"
        );
        assert!(
            !built.holds(&SkyKey::of(&view(40.0, Vec3::new(0.3, 1.0, 0.0)))),
            "a climb"
        );

        let mut brighter = view(2.0, Vec3::new(0.3, 1.0, 0.0));
        brighter.occlusion.w = 0.81;
        assert!(!built.holds(&SkyKey::of(&brighter)), "the brightness of the air");

        let mut moon = view(2.0, Vec3::new(0.3, 1.0, 0.0));
        moon.occluders[1] = Vec4::new(1000.0, 300_000.0, 0.0, 6000.0);
        assert!(!built.holds(&SkyKey::of(&moon)), "an occluder");

        let mut shape = view(2.0, Vec3::new(0.3, 1.0, 0.0));
        shape.air[2].x = 2e-5;
        assert!(!built.holds(&SkyKey::of(&shape)), "the shape of the air");
    }
}
