use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
};

use crate::{
    deps::refs::{main_lock::MainLock, manage::ExistsManaged},
    gm::{
        LossyConvert,
        flat::{Point, Size},
    },
    level::{Level, LevelManager},
    render::{
        BackgroundPipeline, MAX_SPRITE_LIGHTS, PixelBlitPipeline, PolygonPipeline, SpriteBoxPipeline,
        SpriteLight, SpriteView, TerrainPipeline, TexturedSpriteBoxPipeline, WithSamples,
        data::{SpriteInstance, TexturedSpriteInstance},
    },
    ui::{UIManager, ui_drawer::set_viewport},
    window::{
        Window,
        image::{Image, ImageBind, Texture},
        msaa_sample_count,
    },
};

/// Everything that draws a level, built for one sample count.
struct LevelPipelines {
    sprite:     SpriteBoxPipeline,
    textured:   TexturedSpriteBoxPipeline,
    background: BackgroundPipeline,
    polygon:    PolygonPipeline,
    terrain:    TerrainPipeline,
}

impl LevelPipelines {
    fn new(samples: u32) -> Self {
        Self {
            sprite:     SpriteBoxPipeline::with_samples(samples),
            textured:   TexturedSpriteBoxPipeline::with_samples(samples),
            background: BackgroundPipeline::with_samples(samples),
            polygon:    PolygonPipeline::with_samples(samples),
            terrain:    TerrainPipeline::with_samples(samples),
        }
    }
}

/// The pipelines of a level drawn straight into the frame pass.
struct FramePipelines(LevelPipelines);

impl Default for FramePipelines {
    fn default() -> Self {
        Self(LevelPipelines::new(msaa_sample_count()))
    }
}

/// The pipelines of a pixel art level. Its texture has one sample, so an
/// edge covers a texel or does not, no partly covered art pixel.
struct PixelPipelines(LevelPipelines);

impl Default for PixelPipelines {
    fn default() -> Self {
        Self(LevelPipelines::new(1))
    }
}

/// The texture a pixel art level draws into, one texel per art pixel.
struct PixelTarget {
    color: Texture,
    depth: Texture,
    bind:  ImageBind,
}

static FRAME: MainLock<FramePipelines> = MainLock::new();
static PIXEL: MainLock<PixelPipelines> = MainLock::new();
static PIXEL_TARGET: MainLock<Option<PixelTarget>> = MainLock::new();
static BLIT: MainLock<PixelBlitPipeline> = MainLock::new();

/// Where the level of one frame is drawn.
struct Canvas {
    /// The size the view maps to, the render area or the art texture.
    resolution: Size,
    scale:      f32,
    camera:     Point,
}

pub(crate) struct LevelDrawer;

impl LevelDrawer {
    pub(crate) fn update() {
        LevelManager::update();

        // A running level animates every frame, so keep the loop awake while
        // one is loaded. A plain UI screen has no level and stays idle.
        if !LevelManager::no_level() {
            crate::window::request_frame();
        }
    }

    /// Screen pixels per art pixel and the size of the art texture that
    /// covers the render area, even in both sides, so a texel edge falls on
    /// the snapped camera.
    fn pixel_layout(pixel: f32) -> (f32, Size<u32>) {
        let area = UIManager::render_area();
        let screen_pixels = (10.0 * LevelManager::scale() * pixel).round().max(1.0);
        let even = |side: f32| -> u32 {
            let texels: u32 = (side / screen_pixels).ceil().lossy_convert();
            texels + texels % 2 + 2
        };
        (screen_pixels, Size::new(even(area.width), even(area.height)))
    }

    /// A pixel art level draws itself into its art texture before the frame
    /// pass starts.
    pub(crate) fn prepare(encoder: &mut CommandEncoder) {
        if LevelManager::no_level() {
            return;
        }
        let Some(pixel) = LevelManager::pixel_size() else {
            return;
        };
        let (_, size) = Self::pixel_layout(pixel);

        let target = PIXEL_TARGET.get_mut();
        if target.as_ref().is_none_or(|target| target.color.size != size) {
            let color = Texture::pixel_target(size);
            let depth = Texture::create_depth_texture(Window::device(), size, 1, "pixel_art_depth");
            let bind = Image::bind_texture(&color);
            *target = Some(PixelTarget { color, depth, bind });
        }
        let target = target.as_ref().expect("made above");

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label:                    Some("pixel art level"),
            color_attachments:        &[Some(RenderPassColorAttachment {
                view:           &target.color.view,
                depth_slice:    None,
                resolve_target: None,
                ops:            Operations {
                    load:  LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view:        &target.depth.view,
                depth_ops:   Some(Operations {
                    load:  LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: Some(Operations {
                    load:  LoadOp::Clear(0),
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set:      None,
            timestamp_writes:         None,
            multiview_mask:           None,
        });

        let canvas = Canvas {
            resolution: Size::new(size.width.lossy_convert(), size.height.lossy_convert()),
            // The sprite shaders draw ten texels per unit times the scale.
            scale:      1.0 / (10.0 * pixel),
            camera:     LevelManager::view_camera(),
        };
        Self::draw_level(&mut pass, &mut PIXEL.get_mut().0, &canvas);
    }

    pub fn draw(pass: &mut RenderPass) {
        if LevelManager::no_level() {
            return;
        }
        let resolution = UIManager::render_area();
        set_viewport(pass, resolution);

        if let Some(pixel) = LevelManager::pixel_size() {
            Self::show_pixel_art(pass, pixel, resolution);
        } else {
            let canvas = Canvas {
                resolution,
                scale: LevelManager::scale(),
                camera: LevelManager::view_camera(),
            };
            Self::draw_level(pass, &mut FRAME.get_mut().0, &canvas);
        }

        set_viewport(pass, UIManager::window_resolution());
    }

    /// Shows the art texture over the render area, each texel as a square
    /// of whole screen pixels, centered like the level camera.
    fn show_pixel_art(pass: &mut RenderPass, pixel: f32, area: Size) {
        let Some(target) = PIXEL_TARGET.get_mut().as_ref() else {
            return;
        };
        let (screen_pixels, size) = Self::pixel_layout(pixel);
        if target.color.size != size {
            return;
        }
        let (width, height): (f32, f32) = (size.width.lossy_convert(), size.height.lossy_convert());
        let half = Point::new(
            area.width / screen_pixels / width,
            area.height / screen_pixels / height,
        ) / 2.0;
        let middle = Point::new(0.5, 0.5);
        BLIT.get_mut().draw(
            pass,
            &target.bind.bind,
            middle - half,
            middle + half,
            LevelManager::default_z_position(),
        );
    }

    fn draw_level(pass: &mut RenderPass, pipelines: &mut LevelPipelines, canvas: &Canvas) {
        let level = LevelManager::level();

        if level.background.is_ok() {
            pipelines.background.draw(
                pass,
                &level.background,
                canvas.resolution,
                canvas.camera.neg() / 10.0,
                0.0,
                canvas.scale,
            );
        }

        pipelines.polygon.clear();

        Self::add_tile_maps(level, &mut pipelines.textured);

        for sprite in level.sprites() {
            if sprite.image.exists_managed() {
                let mut instance = TexturedSpriteInstance::new(
                    sprite.position(),
                    sprite.render_size(),
                    sprite.rotation(),
                    sprite.z_position,
                )
                .flipped(sprite.flip.x, sprite.flip.y);
                instance.scale = sprite.image_scale;
                instance.tint = sprite.tint;
                pipelines.textured.add_with_image(instance, sprite.image);
            } else if let Some(vertex_buffer) = &sprite.vertex_buffer {
                pipelines.polygon.add(
                    vertex_buffer,
                    sprite.position(),
                    *sprite.color(),
                    sprite.rotation(),
                );
            } else {
                pipelines.sprite.add(SpriteInstance {
                    size:       sprite.render_size(),
                    position:   sprite.position(),
                    color:      *sprite.color(),
                    rotation:   sprite.rotation(),
                    z_position: sprite.z_position,
                });
            }
        }

        let view = Self::sprite_view(level, canvas);

        // The ground goes before the sprites in front of it, so their soft
        // edges blend over it and not over the clear color.
        pipelines
            .terrain
            .draw(pass, &view, level.terrains(), LevelManager::visible_rect());
        pipelines.sprite.draw(pass, view);
        pipelines.textured.sort_back_to_front(|instance| instance.z_position);
        pipelines.textured.draw(pass, view);
        pipelines.polygon.draw(pass, view);
    }

    /// Only the cells on screen are drawn, a long run of tiles costs what
    /// one screen of it does.
    fn add_tile_maps(level: &dyn Level, textured: &mut TexturedSpriteBoxPipeline) {
        let visible = LevelManager::visible_rect();
        for map in level.tile_maps() {
            for (cell, image) in map.visible_cells(visible) {
                textured.add_with_image(
                    TexturedSpriteInstance::new(cell.center(), cell.size / 2.0, 0.0, map.z_position),
                    image,
                );
            }
        }
    }

    /// The camera and the light of the frame. With more lights than the
    /// shader takes, the ones nearest the screen win.
    fn sprite_view(level: &dyn Level, canvas: &Canvas) -> SpriteView {
        let mut view = SpriteView {
            camera_pos: canvas.camera,
            resolution: canvas.resolution,
            scale: canvas.scale,
            ambient: level.ambient_light,
            ..Default::default()
        };

        let visible = LevelManager::visible_rect();
        let center = visible.center();
        let mut lights: Vec<_> = level
            .lights()
            .iter()
            .filter(|light| light.radius > 0.0 && light.intensity > 0.0)
            .map(|light| ((light.position - center).length() - light.radius, light))
            .collect();
        if lights.len() > MAX_SPRITE_LIGHTS {
            lights.sort_by(|a, b| a.0.total_cmp(&b.0));
            lights.truncate(MAX_SPRITE_LIGHTS);
        }

        for (slot, (_, light)) in view.lights.iter_mut().zip(&lights) {
            *slot = SpriteLight {
                color:    light.color.with_alpha(light.intensity),
                position: light.position,
                radius:   light.radius,
                falloff:  light.falloff,
            };
        }
        view.light_count = u32::try_from(lights.len()).expect("at most MAX_SPRITE_LIGHTS lights");

        view
    }
}
