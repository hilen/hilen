#[cfg(all(windows, feature = "xr"))]
mod eye;

use std::array::from_fn;

use glam::DMat4;
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureView, TextureViewDescriptor,
};

use crate::{
    deps::refs::{Weak, main_lock::MainLock, manage::DataManager},
    gm::{
        LossyConvert,
        color::{Color, WHITE},
        flat::Size,
        volume::{Bounds, Mat3, Mat4, Quat, Shape3, Vec3, Vec4},
    },
    image::{Image, ImageBind, Texture},
    render::{
        MeshKey, MeshPipeline, PicturePipeline, Reach, SceneUniform,
        data::{MeshInstance, WideLine},
        frustum::{self, Frustum},
    },
    scene::{
        Atmosphere, Camera, DayNightSky, Fog, LightPick, Line3, MAX_OCCLUDERS, Material, Mesh, Model, Node,
        Playback, Projection, Scene, SceneManager, SpaceSky, collider_lines, pick_lights, sun_cascades,
    },
    ui::{
        UIManager,
        ui_drawer::{UIDrawer, set_viewport},
    },
    window::{Window, msaa_sample_count, state::State, surface_texture_format},
};

static MESH: MainLock<MeshPipeline> = MainLock::new();
/// The pipelines of a scene under a render scale whose sample count is not
/// the frame's, see `SceneManager::set_scene_samples`. Made at the first
/// such frame, an app that never asks for it does not wait for them.
static MESH_OWN_SAMPLES: MainLock<MeshPipeline> = MainLock::new();
static PICTURE: MainLock<PicturePipeline> = MainLock::new();
static PICTURE_TARGETS: MainLock<Option<PictureTargets>> = MainLock::new();
/// The targets of the scene in a `SceneView`, apart from the pictures so
/// a grid of pictures next to the view does not make them again.
static VIEW_TARGETS: MainLock<Option<PictureTargets>> = MainLock::new();
/// The RGBA texture a photo is read back from, kept for the next photo
/// of the same size, see `SceneManager::photo`.
static PHOTO: MainLock<Option<Texture>> = MainLock::new();
/// The picture of a scene over the whole window at a render scale under 1,
/// see `SceneManager::set_render_scale`, and the targets of its size.
/// The flag is whether the image is in the surface format, see
/// `window_picture`.
static WINDOW_PICTURE: MainLock<Option<(Weak<Image>, bool)>> = MainLock::new();
static WINDOW_TARGETS: MainLock<Option<PictureTargets>> = MainLock::new();

/// Drops what is built for a sample count. Each part is made again when
/// it is next used.
pub(crate) fn drop_pipelines() {
    MESH.clear();
    MESH_OWN_SAMPLES.clear();
    PICTURE_TARGETS.clear();
    VIEW_TARGETS.clear();
    WINDOW_TARGETS.clear();
}

/// The targets of `size` with `samples` a pixel in `slot`, made again for
/// another size or count.
fn targets_of(slot: &mut Option<PictureTargets>, size: Size<u32>, samples: u32) -> &PictureTargets {
    if slot
        .as_ref()
        .is_none_or(|targets| targets.size != size || targets.samples != samples)
    {
        *slot = Some(PictureTargets::new(size, samples));
    }
    slot.as_ref().expect("just set")
}

/// The mesh pipelines for a pass of `samples` samples a pixel.
fn mesh_for(samples: u32) -> &'static mut MeshPipeline {
    if samples == msaa_sample_count() {
        MESH.get_mut()
    } else {
        let pipeline = MESH_OWN_SAMPLES.get_or_init(|| MeshPipeline::new(samples));
        if pipeline.samples() == samples {
            pipeline
        } else {
            MESH_OWN_SAMPLES.set(MeshPipeline::new(samples))
        }
    }
}

/// What a picture is drawn into before it becomes an image, kept for the
/// next picture of the same size, so a grid of pictures makes them once.
struct PictureTargets {
    size:     Size<u32>,
    samples:  u32,
    /// The multisampled color, none when multisampling is off.
    msaa:     Option<TextureView>,
    depth:    Texture,
    /// Where the samples resolve to, in the surface format.
    resolved: Texture,
    bind:     ImageBind,
}

impl PictureTargets {
    fn new(size: Size<u32>, samples: u32) -> Self {
        let device = Window::device();
        let msaa = (samples > 1).then(|| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label:           Some("picture_msaa"),
                    size:            wgpu::Extent3d {
                        width:                 size.width,
                        height:                size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count:    samples,
                    dimension:       wgpu::TextureDimension::D2,
                    format:          surface_texture_format(),
                    usage:           wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats:    &[],
                })
                .create_view(&TextureViewDescriptor::default())
        });
        let resolved = Texture::pixel_target(size);
        Self {
            size,
            samples,
            msaa,
            depth: Texture::create_depth_texture(device, size, samples, "picture_depth"),
            bind: Image::bind_texture(&resolved),
            resolved,
        }
    }
}

const COLLIDER_COLOR: Color = Color::hex("#00ff60");

/// The scene draws in the whole depth range, reversed. The frame's depth
/// buffer is the UI's too, so the scene fills it with 0 before it draws
/// and with 1 after, and the UI over it tests depth as before, see
/// `MeshPipeline::fill_depth`. A band of the range lost the reversed
/// depth its gain: floats are dense only near 0.
const DEPTH_BAND: (f32, f32) = (0.0, 1.0);

/// How `draw_into` gets the scene into its target.
#[derive(Debug, Clone, Copy)]
enum Route {
    /// Through the picture copy, filled by this.
    Copy(Fill),
    /// The samples resolve straight into a target in the surface format,
    /// over an opaque clear.
    Straight,
}

/// What a picture of the scene holds.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Fill {
    /// The lit scene over this color where nothing is drawn.
    Light(Color),
    /// The distance of every pixel, see `SceneManager::depth_photo`.
    Depth,
}

pub(crate) struct SceneDrawer;

fn push_line(pipeline: &mut MeshPipeline, line: Line3) {
    pipeline.add_wide_line(
        WideLine::new(line.from, line.to, line.color, line.width),
        line.on_top,
    );
}

/// Where one node's draws go: the batch of its mesh and textures, or the
/// translucent list drawn after every batch.
struct NodeDraws<'a> {
    pipeline:    &'a mut MeshPipeline,
    /// The camera's view, a translucent node out of it is left out. `None`
    /// draws every node, see `frustum::enabled`.
    frustum:     Option<&'a Frustum>,
    camera:      Vec3,
    translucent: &'a mut Vec<(f32, MeshKey, MeshInstance)>,
    lights:      LightPick,
    distance:    f32,
    /// The sphere around the node, what a pass that cannot see it leaves
    /// out by.
    reach:       Reach,
}

impl NodeDraws<'_> {
    /// `posed` is a draw that can leave its node's sphere, a skinned part
    /// or a part of a playing clip, the rest bounds do not hold its pose,
    /// so every pass draws it.
    fn push(&mut self, mesh: Weak<Mesh>, model: Mat4, material: Material, joint_base: u32, posed: bool) {
        if material.fade <= 0.0 {
            return;
        }
        // A fading node leaves out pixels, the cutout pipelines do that.
        let cutout = material.cutout.is_some() || material.fade < 1.0;
        let key = MeshKey {
            mesh,
            texture: material.texture,
            normal_map: material.normal_map,
            metallic_roughness_map: material.metallic_roughness_map,
            occlusion_map: material.occlusion_map,
            flow_map: material.flow_map,
            cutout,
            triplanar: material.triplanar > 0.0,
        };
        let instance = MeshInstance::new(model, material, self.lights, joint_base);
        // A cut out node is solid where it is drawn, whatever its alpha.
        if !cutout && material.color.a < 1.0 {
            // The same test `MeshPipeline::prepare` gives an opaque node, a
            // posed one can leave its sphere.
            let seen = posed
                || self
                    .frustum
                    .is_none_or(|frustum| frustum.holds(self.reach.center - self.camera, self.reach.radius));
            if seen {
                self.translucent.push((self.distance, key, instance));
            }
        } else {
            self.pipeline.add(key, instance, (!posed).then_some(self.reach));
        }
    }

    /// The collider of a node as a wireframe at the collider's own center,
    /// see `collider_lines`. The slab a plane collides with by default is
    /// the drawn floor itself and gets no lines.
    fn push_collider(&mut self, node: &dyn Node, center: Vec3) {
        if node.collider.is_none() && matches!(node.shape, Shape3::Plane(_)) {
            return;
        }
        let (shape, _) = node.collider_shape();
        for (from, to) in collider_lines(&shape, center, node.rotation(), node.world_scale()) {
            self.pipeline.add_line(from, to, COLLIDER_COLOR);
        }
    }

    /// Every part of a model at the pose its node plays, or at rest.
    /// A skinned part draws through its joint matrices, queued once per
    /// skin, and an unskinned one at its node's posed place.
    fn push_model(
        &mut self,
        model: &Model,
        model_matrix: Mat4,
        material: Material,
        playback: Option<Playback>,
    ) {
        let posed = model
            .rig
            .as_ref()
            .zip(playback)
            .map(|(rig, playback)| rig.pose(Some((&rig.clips[playback.clip], playback.time))));
        let mut bases: Vec<Option<u32>> = vec![None; model.rest_joints.len()];

        for part in &model.parts {
            let (transform, joint_base) = match part.skin {
                Some(skin) => {
                    let base = *bases[skin].get_or_insert_with(|| match (&posed, &model.rig) {
                        (Some(globals), Some(rig)) => {
                            self.pipeline.add_joints(&rig.skins[skin].joint_matrices(globals))
                        }
                        _ => self.pipeline.add_joints(&model.rest_joints[skin]),
                    });
                    (Mat4::IDENTITY, base)
                }
                None => (
                    posed.as_ref().map_or(part.transform, |globals| globals[part.node]),
                    0,
                ),
            };
            self.push(
                part.mesh.weak(),
                model_matrix * transform,
                part.material.unwrap_or(material),
                joint_base,
                part.skin.is_some() || posed.is_some(),
            );
        }
    }
}

impl SceneDrawer {
    pub(crate) fn update() {
        SceneManager::update();

        // A running scene animates every frame like a level does.
        if !SceneManager::no_scene() {
            crate::window::request_frame();
        }
    }

    /// Gathers the frame's nodes into the pipeline, loads them and draws
    /// the shadow map, before the frame's pass opens. A scene in a
    /// `SceneView` draws its whole picture here, into the image of the
    /// view, which the frame's pass then draws like any image.
    pub(crate) fn prepare(encoder: &mut CommandEncoder) {
        if SceneManager::no_scene() {
            return;
        }

        if let Some(host) = SceneManager::host() {
            if host.is_ok()
                && let Some(image) = host.frame_target()
            {
                Self::draw_live_picture(
                    image,
                    VIEW_TARGETS.get_mut(),
                    msaa_sample_count(),
                    host.background(),
                    encoder,
                );
            }
            return;
        }

        if let Some(size) = Self::window_picture_size() {
            // Over an opaque clear every pixel has an alpha of 1, nothing to
            // divide out, so the samples resolve straight into the image the
            // frame stretches and the copy pass of a picture is left out.
            let background = Window::clear_color();
            let straight = background.a >= 1.0;
            let image = Self::window_picture(size, straight);
            let targets = targets_of(WINDOW_TARGETS.get_mut(), size, SceneManager::scene_samples());
            let scene = SceneManager::scene();
            let route = if straight {
                Route::Straight
            } else {
                Route::Copy(Fill::Light(background))
            };
            Self::draw_into(scene, &scene.camera, image.view(), size, targets, route, encoder);
            return;
        }

        let scene = SceneManager::scene();
        Self::gather(
            MESH.get_mut(),
            scene,
            &scene.camera,
            UIManager::render_area(),
            DEPTH_BAND,
            encoder,
        );
    }

    /// Draws the running scene into `image` through its own camera, the
    /// picture of a `SceneView` or of a scene under a render scale, with
    /// the targets kept in `slot` for the next frame.
    fn draw_live_picture(
        image: Weak<Image>,
        slot: &mut Option<PictureTargets>,
        samples: u32,
        background: Color,
        encoder: &mut CommandEncoder,
    ) {
        let targets = targets_of(slot, image.size, samples);
        let scene = SceneManager::scene();
        Self::draw_into(
            scene,
            &scene.camera,
            image.view(),
            image.size,
            targets,
            Route::Copy(Fill::Light(background)),
            encoder,
        );
    }

    /// The pixels of the picture of a scene over the whole window, `None`
    /// when it draws straight into the frame.
    fn window_picture_size() -> Option<Size<u32>> {
        let scale = SceneManager::render_scale();
        if scale >= 1.0 {
            return None;
        }
        let area = UIManager::render_area();
        let size: Size<u32> = Size::new(
            (area.width * scale).round().lossy_convert(),
            (area.height * scale).round().lossy_convert(),
        );
        (size.width > 0 && size.height > 0).then_some(size)
    }

    /// The image of `size` pixels, made again when the size changed. A
    /// `straight` one is in the format of the surface, the samples resolve
    /// into it, any other gets the picture copy with its alpha divided out.
    fn window_picture(size: Size<u32>, straight: bool) -> Weak<Image> {
        let slot = WINDOW_PICTURE.get_mut();
        if let Some((image, _)) = slot
            .as_ref()
            .filter(|(image, kind)| image.is_ok() && image.size == size && *kind == straight)
        {
            return *image;
        }
        let name = format!(
            "scene-window-{}x{}{}",
            size.width,
            size.height,
            if straight { "-straight" } else { "" }
        );
        Self::free_window_picture();
        let image = if straight {
            Image::surface_target(&name, size)
        } else {
            Image::render_target(&name, size)
        };
        *WINDOW_PICTURE.get_mut() = Some((image, straight));
        image
    }

    /// Frees the picture of a stopped scene, it holds a window of pixels.
    pub(crate) fn free_window_picture() {
        if let Some((image, _)) = WINDOW_PICTURE.get_mut().take().filter(|(image, _)| image.is_ok()) {
            image.free();
        }
        WINDOW_TARGETS.clear();
    }

    /// Gathers the nodes of `scene` seen through `camera` into the
    /// pipeline for a target of `area` pixels that draws in the depth band
    /// `depth`, loads them and draws the shadow map.
    fn gather(
        pipeline: &mut MeshPipeline,
        scene: &dyn Scene,
        camera: &Camera,
        area: Size,
        depth: (f32, f32),
        encoder: &mut CommandEncoder,
    ) {
        for light in &scene.lights {
            pipeline.add_light(light.mesh_light());
        }

        let aspect = area.width / area.height;
        let view_proj = camera.view_projection(aspect);
        // The camera's space with its origin on the camera, see
        // `camera_offset` in scene_common.wgsl. Turned in 64 bit, the
        // camera's own place cancels out exactly.
        let relative = view_proj.as_dmat4() * DMat4::from_translation(camera.position.as_dvec3());
        let frustum = Frustum::of(relative.as_mat4());

        let extent = Self::gather_nodes(
            pipeline,
            scene,
            camera,
            area,
            frustum::enabled().then_some(&frustum),
        );

        let sun = scene.sun;
        let sun_color = sun.color.linear();
        let ambient = scene.ambient.linear();
        let sky = scene.sky.as_ref();
        let space = scene.space_sky;
        let air = space.and_then(|space| space.atmosphere);
        let air_brightness = space.map_or(1.0, |space| space.air_brightness);
        pipeline.air.update(air.as_ref());
        let sun_light = Vec3::new(sun_color.r, sun_color.g, sun_color.b) * sun.intensity;
        let air_ambient = air.and_then(|air| {
            let to_sun = -sun.direction.normalize_or_zero();
            pipeline.air.ambient(&air, camera.position - air.center, to_sun)
        });
        let [sky_sun, sky_moon, sky_params] = DayNightSky::shader_data(scene.day_night_sky.as_ref());
        let (fog_color, fog_range) = fog_data(scene.fog);
        let cascades = sun_cascades(&sun, camera, aspect, Bounds::of_points(extent));
        let mut sun_texel = Vec4::ZERO;
        let mut sun_depth = Vec4::ZERO;
        for (slot, cascade) in cascades.iter().enumerate() {
            sun_texel[slot] = cascade.texel;
            sun_depth[slot] = 1.0 / cascade.depth;
        }

        for grass in &scene.grass {
            grass.push_draws(pipeline, camera.position, &frustum, &scene.lights);
        }
        // Each field of grass lays the wind onto its own ground, see
        // `v_grass`.
        let wind = scene.wind.direction.normalize_or_zero();
        let grass_push = scene
            .grass_pusher
            .map_or(Vec4::ZERO, |feet| (feet - camera.position).extend(1.0));
        let grass_trail = scene
            .grass_trail
            .map_or(Vec4::ZERO, |trail| (trail - camera.position).extend(0.0));
        let view = SceneUniform {
            view_proj: relative.as_mat4(),
            inv_view_proj: relative.inverse().as_mat4(),
            sun_view_proj: cascades.map(|cascade| cascade.view_proj),
            camera_pos: camera.position.extend(2.0 * (camera.fov_y / 2.0).tan()),
            eye: camera.forward().extend(match camera.projection {
                Projection::Perspective | Projection::Fov(_) => 0.0,
                Projection::Orthographic { height } => height,
            }),
            sun_dir: sun.direction.normalize_or_zero().extend(0.0),
            sun_color: Vec4::new(
                sun_color.r * sun.intensity,
                sun_color.g * sun.intensity,
                sun_color.b * sun.intensity,
                f32::from(u8::from(sun.shadows)),
            ),
            ambient: Vec4::new(
                ambient.r,
                ambient.g,
                ambient.b,
                f32::from(u8::from(sky.is_some() || air_ambient.is_some())),
            ),
            viewport: Vec4::new(area.width, area.height, depth.0, depth.1 - depth.0),
            sun_texel,
            sun_depth,
            fog_color,
            fog_range,
            sky_sun,
            sky_moon,
            sky_params,
            irradiance: match air_ambient {
                Some(coefficients) => coefficients.map(|c| (c * sun_light * air_brightness).extend(0.0)),
                None => sky.map_or([Vec4::ZERO; 9], |sky| sky.irradiance),
            },
            grass_wind: wind.extend(scene.wind.strength),
            grass_push,
            grass_trail,
            occluders: occluder_balls(scene, camera),
            transmit: occluder_colors(scene),
            occlusion: Vec4::new(
                scene.occluders.len().min(MAX_OCCLUDERS).lossy_convert(),
                sun.angular_radius,
                space.as_ref().map_or(0.0, |space| space.sun_glare),
                air_brightness,
            ),
            air: air_data(air.as_ref(), camera),
            space: space_data(space.as_ref()),
            stars: star_turn(space.as_ref()),
            flow: Vec4::new(scene.flow_time, 0.0, 0.0, 0.0),
        };

        pipeline.prepare(encoder, &view, sun.shadows, sun.shadow_map_size);
    }

    /// Puts every node, the lines and the handle of `scene` into the
    /// pipeline. Translucent nodes blend over what is behind them, so they
    /// go after every opaque node, the farthest first. Gives the corners
    /// of every node's sphere, what the shadow cascades stay inside of.
    fn gather_nodes(
        pipeline: &mut MeshPipeline,
        scene: &dyn Scene,
        camera: &Camera,
        area: Size,
        frustum: Option<&Frustum>,
    ) -> Vec<Vec3> {
        let mut translucent = vec![];
        let mut extent = vec![];
        pipeline.wireframe = scene.show_wireframe;

        for node in scene.nodes() {
            let shape = node.shape();
            let (position, rotation) = pose(&**node);
            let scale = node.world_scale();
            // The drawn solid's center, a model's origin can sit off its
            // bounds. The collider can sit elsewhere, see `set_collider`.
            let stretch = scale * node.mesh_scale;
            let center = position + rotation * (shape.collider_offset() * stretch);
            let reach = (shape.half_extents() * stretch).length();
            let model_matrix =
                Mat4::from_scale_rotation_translation(shape.mesh_scale() * stretch, rotation, position);
            extent.push(center - reach);
            extent.push(center + reach);

            let mut draws = NodeDraws {
                pipeline,
                frustum,
                camera: camera.position,
                translucent: &mut translucent,
                lights: pick_lights(center, reach, &scene.lights),
                distance: camera.position.distance_squared(center),
                reach: Reach {
                    center,
                    radius: reach,
                },
            };

            match shape {
                Shape3::Model(model) if model.is_ok() => {
                    draws.push_model(&model, model_matrix, node.material, node.playback);
                }
                _ => {
                    if let Some(mesh) = node.mesh {
                        draws.push(mesh, model_matrix, node.material, 0, false);
                    }
                }
            }

            if scene.show_colliders && node.collider_handle().is_some() {
                draws.push_collider(
                    &**node,
                    node.position() + node.rotation() * node.collider_offset(),
                );
            }

            if !node.lines.is_empty() {
                let space = Mat4::from_scale_rotation_translation(Vec3::splat(scale), rotation, position);
                for line in &node.lines {
                    push_line(pipeline, line.transformed(space));
                }
            }
        }

        for line in &scene.lines {
            push_line(pipeline, *line);
        }
        if let Some(handle) = &scene.handle {
            for line in handle.lines(camera, area) {
                push_line(pipeline, line);
            }
        }

        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, key, instance) in translucent {
            pipeline.add_transparent(key, instance);
        }
        extent
    }

    pub fn draw(pass: &mut RenderPass) {
        if SceneManager::no_scene() || SceneManager::host().is_some() {
            return;
        }

        if Self::window_picture_size().is_some() {
            if let Some((image, _)) = *WINDOW_PICTURE.get_mut() {
                UIDrawer::draw_behind_views(pass, image);
            }
            return;
        }

        let area = UIManager::render_area();
        pass.set_viewport(0.0, 0.0, area.width, area.height, DEPTH_BAND.0, DEPTH_BAND.1);

        let scene = SceneManager::scene();
        let pipeline = MESH.get_mut();
        pipeline.fill_depth(pass, true);
        pipeline.draw(
            pass,
            scene.sky.as_ref(),
            scene.sky.is_some()
                || scene.day_night_sky.is_some()
                || scene.space_sky.is_some()
                || scene.fog.is_some(),
        );
        pipeline.fill_depth(pass, false);

        set_viewport(pass, UIManager::window_resolution());
    }

    /// Draws `scene` once into the image `name` of `size` pixels, over
    /// `background`. Its own encoder and its own submit, so the frame on
    /// screen and the running scene are not touched, the mesh pipeline is
    /// shared and left empty for the frame's own draw.
    pub(crate) fn picture(scene: &dyn Scene, name: &str, size: Size<u32>, background: Color) -> Weak<Image> {
        assert!(
            size.width > 0 && size.height > 0,
            "A scene picture needs a size, got {size:?}"
        );

        let image = Image::render_target(&format!("{name}-{}x{}", size.width, size.height), size);
        let targets = targets_of(PICTURE_TARGETS.get_mut(), size, msaa_sample_count());

        let mut encoder = Window::device().create_command_encoder(&CommandEncoderDescriptor {
            label: Some("scene_picture"),
        });

        Self::draw_into(
            scene,
            &scene.camera,
            image.view(),
            image.size,
            targets,
            Route::Copy(Fill::Light(background)),
            &mut encoder,
        );

        crate::render::upload_pending();
        Window::queue().submit([encoder.finish()]);
        MESH.get_mut().forget_frame();

        image
    }

    /// Draws `scene` once through `camera` into an RGBA picture of `size`
    /// pixels filled by `fill`, the way `picture` does, and copies it into
    /// a buffer to read back, see `SceneManager::photo` and
    /// `SceneManager::depth_photo`. The scene's own camera and the frame
    /// on screen are not touched.
    pub(crate) fn photo(scene: &dyn Scene, camera: &Camera, size: Size<u32>, fill: Fill) -> wgpu::Buffer {
        let photo = PHOTO.get_mut();
        if photo.as_ref().is_none_or(|texture| texture.size != size) {
            *photo = Some(Texture::render_target(size, "scene_photo"));
        }
        let photo = photo.as_ref().expect("just set");
        let targets = targets_of(PICTURE_TARGETS.get_mut(), size, msaa_sample_count());

        let mut encoder = Window::device().create_command_encoder(&CommandEncoderDescriptor {
            label: Some("scene_photo"),
        });

        Self::draw_into(
            scene,
            camera,
            &photo.view,
            size,
            targets,
            Route::Copy(fill),
            &mut encoder,
        );
        let (buffer, _) = State::read_screen(&mut encoder, &photo.texture);

        crate::render::upload_pending();
        Window::queue().submit([encoder.finish()]);
        MESH.get_mut().forget_frame();

        buffer
    }

    /// Draws `scene` through `camera` into `target` of `size` pixels with
    /// `targets` of that size, filled by `fill`, with the camera aspect
    /// of the target.
    fn draw_into(
        scene: &dyn Scene,
        camera: &Camera,
        target: &TextureView,
        size: Size<u32>,
        targets: &PictureTargets,
        route: Route,
        encoder: &mut CommandEncoder,
    ) {
        let fill = match route {
            Route::Copy(fill) => fill,
            Route::Straight => Fill::Light(Window::clear_color()),
        };
        // White is the 24 bits a depth photo keeps for no surface.
        let background = match fill {
            Fill::Light(color) => color,
            Fill::Depth => WHITE,
        };
        let area: Size = (size.width.lossy_convert(), size.height.lossy_convert()).into();

        let pipeline = mesh_for(targets.samples);
        Self::gather(pipeline, scene, camera, area, (0.0, 1.0), encoder);

        let resolved = match route {
            Route::Copy(_) => &targets.resolved.view,
            Route::Straight => target,
        };
        let (color, resolve_target) = match &targets.msaa {
            Some(msaa) => (msaa, Some(resolved)),
            None => (resolved, None),
        };
        // Color times alpha, what a blended node leaves in the frame too,
        // the picture pass divides it back out.
        let alpha = f64::from(background.a);
        let clear = wgpu::Color {
            r: f64::from(background.r) * alpha,
            g: f64::from(background.g) * alpha,
            b: f64::from(background.b) * alpha,
            a: alpha,
        };

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label:                    Some("Scene Picture Pass"),
                color_attachments:        &[Some(RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target,
                    ops: Operations {
                        load:  LoadOp::Clear(clear),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view:        &targets.depth.view,
                    depth_ops:   Some(Operations {
                        // The far end of the scene's reversed depth.
                        load:  LoadOp::Clear(0.0),
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: Some(Operations {
                        load:  LoadOp::Clear(0),
                        store: StoreOp::Discard,
                    }),
                }),
                occlusion_query_set:      None,
                timestamp_writes:         None,
                multiview_mask:           None,
            });
            pass.set_viewport(0.0, 0.0, area.width, area.height, 0.0, 1.0);
            match fill {
                Fill::Light(_) => pipeline.draw(
                    &mut pass,
                    scene.sky.as_ref(),
                    scene.sky.is_some()
                        || scene.day_night_sky.is_some()
                        || scene.space_sky.is_some()
                        || scene.fog.is_some(),
                ),
                Fill::Depth => pipeline.draw_depth(&mut pass, scene.sky.as_ref()),
            }
        }

        if let Route::Copy(_) = route {
            PICTURE.get_mut().draw(encoder, &targets.bind.bind, target);
        }
    }
}

/// Where `node` stands and how it is turned. A node with no body and no
/// parent keeps both in its own fields, every move of its collider goes
/// through `set_position`, `set_rotation` or `set_pose`, which write them
/// too. Read from rapier, 3 or 4 times a node, it was a lookup into the
/// physics for every wall and prop of a streamed world every frame.
fn pose(node: &dyn Node) -> (Vec3, Quat) {
    if node.rigid_handle().is_none() && node.parent().is_none() {
        (node.position, node.rotation)
    } else {
        (node.position(), node.rotation())
    }
}

/// The color of the fog and its range as `SceneUniform` holds them, all 0
/// with no fog.
fn fog_data(fog: Option<Fog>) -> (Vec4, Vec4) {
    fog.map_or((Vec4::ZERO, Vec4::ZERO), |fog| {
        let color = fog.color.linear();
        let (start, inverse) = fog.range();
        (
            Vec4::new(color.r, color.g, color.b, 1.0),
            Vec4::new(start, inverse, fog.height.max(1e-3), 0.0),
        )
    })
}

/// The occluders from the camera with their radii, for `light_through`.
fn occluder_balls(scene: &dyn Scene, camera: &Camera) -> [Vec4; MAX_OCCLUDERS] {
    from_fn(|index| {
        scene.occluders.get(index).map_or(Vec4::ZERO, |occluder| {
            (occluder.center - camera.position).extend(occluder.radius)
        })
    })
}

/// The light each occluder lets through, in linear light.
fn occluder_colors(scene: &dyn Scene) -> [Vec4; MAX_OCCLUDERS] {
    from_fn(|index| {
        scene.occluders.get(index).map_or(Vec4::ZERO, |occluder| {
            let light = occluder.transmit.linear();
            Vec4::new(light.r, light.g, light.b, 0.0)
        })
    })
}

/// The air in the 6 vectors `atmosphere.wgsl` reads, its center from the
/// camera, all 0 with no air.
fn air_data(air: Option<&Atmosphere>, camera: &Camera) -> [Vec4; 6] {
    air.map_or([Vec4::ZERO; 6], |air| {
        [
            (air.center - camera.position).extend(air.ground_radius),
            air.rayleigh_scattering.extend(air.top_radius),
            air.mie_scattering.extend(air.rayleigh_height),
            air.mie_extinction.extend(air.mie_height),
            air.ozone_absorption.extend(air.ozone_center),
            Vec4::new(air.ozone_width, air.mie_g, air.ground_albedo, 1.0),
        ]
    })
}

fn space_data(space: Option<&SpaceSky>) -> Vec4 {
    space.map_or(Vec4::ZERO, |space| {
        Vec4::new(
            1.0,
            space.star_brightness,
            space.star_seed,
            f32::from(u8::from(space.sun_disc)),
        )
    })
}

/// The turn from the world to the stars' own axes.
fn star_turn(space: Option<&SpaceSky>) -> [Vec4; 3] {
    let turn = Mat3::from_quat(space.map_or(Quat::IDENTITY, |space| space.stars).inverse());
    [
        turn.x_axis.extend(0.0),
        turn.y_axis.extend(0.0),
        turn.z_axis.extend(0.0),
    ]
}
