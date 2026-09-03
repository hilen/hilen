use wgpu::{CommandEncoder, RenderPass};

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::volume::{Bounds, Mat4, Shape3, Vec4},
    render::{MeshKey, MeshPipeline, SceneView, data::MeshInstance},
    scene::{LightPick, Material, Mesh, Model, Playback, SceneManager, pick_lights, sun_cascades},
    ui::{UIManager, ui_drawer::set_viewport},
};

static MESH: MainLock<MeshPipeline> = MainLock::new();

/// The scene owns this band of the frame's shared depth buffer. The UI
/// draws at 0.5 and closer, a level sprite at 0.85, so the UI stays in
/// front of any scene and a level can share the frame. The band costs
/// about one and a half bits of depth precision, the near plane of the
/// camera matters far more.
const DEPTH_BAND: (f32, f32) = (0.6, 1.0);

pub(crate) struct SceneDrawer;

/// Where one node's draws go: the batch of its mesh and textures, or the
/// translucent list drawn after every batch.
struct NodeDraws<'a> {
    pipeline:    &'a mut MeshPipeline,
    translucent: &'a mut Vec<(f32, MeshKey, MeshInstance)>,
    lights:      LightPick,
    distance:    f32,
}

impl NodeDraws<'_> {
    fn push(&mut self, mesh: Weak<Mesh>, model: Mat4, material: Material, joint_base: u32) {
        let key = MeshKey {
            mesh,
            texture: material.texture,
            normal_map: material.normal_map,
        };
        let instance = MeshInstance::new(model, material, self.lights, joint_base);
        if material.color.a < 1.0 {
            self.translucent.push((self.distance, key, instance));
        } else {
            self.pipeline.add(key, instance);
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
    /// the shadow map, before the frame's pass opens.
    pub(crate) fn prepare(encoder: &mut CommandEncoder) {
        if SceneManager::no_scene() {
            return;
        }

        let area = UIManager::render_area();
        let scene = SceneManager::scene();
        let pipeline = MESH.get_mut();
        let camera = scene.camera;

        for light in &scene.lights {
            pipeline.add_light(light.mesh_light());
        }

        // Translucent nodes blend over what is behind them, so they draw
        // after every opaque node, the farthest first.
        let mut translucent = vec![];
        // The corners of every node's sphere, what the shadow cascades
        // stay inside of.
        let mut extent = vec![];

        for node in scene.nodes() {
            let shape = node.shape();
            // The solid's center, a model's origin can sit off its bounds.
            let center = node.position() + node.rotation() * node.collider_offset();
            let reach = node.half_extents().length();
            let model_matrix = node.model_matrix();
            extent.push(center - reach);
            extent.push(center + reach);

            let mut draws = NodeDraws {
                pipeline,
                translucent: &mut translucent,
                lights: pick_lights(center, reach, &scene.lights),
                distance: camera.position.distance_squared(center),
            };

            match shape {
                Shape3::Model(model) if model.is_ok() => {
                    draws.push_model(&model, model_matrix, node.material, node.playback);
                }
                _ => {
                    if let Some(mesh) = node.mesh {
                        draws.push(mesh, model_matrix, node.material, 0);
                    }
                }
            }
        }

        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, key, instance) in translucent {
            pipeline.add_transparent(key, instance);
        }

        let sun = scene.sun;
        let sun_color = sun.color.linear();
        let ambient = scene.ambient.linear();
        let sky = scene.sky.as_ref();
        let fog = scene.fog;
        let fog_color = fog.map_or(Vec4::ZERO, |fog| {
            let color = fog.color.linear();
            Vec4::new(color.r, color.g, color.b, 1.0)
        });
        let fog_range = fog.map_or(Vec4::ZERO, |fog| {
            let (start, inverse) = fog.range();
            Vec4::new(start, inverse, fog.height.max(1e-3), 0.0)
        });
        let aspect = area.width / area.height;
        let view_proj = camera.view_projection(aspect);
        let cascades = sun_cascades(&sun, &camera, aspect, Bounds::of_points(extent));
        let mut sun_texel = Vec4::ZERO;
        let mut sun_depth = Vec4::ZERO;
        for (slot, cascade) in cascades.iter().enumerate() {
            sun_texel[slot] = cascade.texel;
            sun_depth[slot] = 1.0 / cascade.depth;
        }

        let view = SceneView {
            view_proj,
            inv_view_proj: view_proj.inverse(),
            sun_view_proj: cascades.map(|cascade| cascade.view_proj),
            camera_pos: camera.position.extend(2.0 * (camera.fov_y / 2.0).tan()),
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
                f32::from(u8::from(sky.is_some())),
            ),
            viewport: Vec4::new(area.width, area.height, DEPTH_BAND.0, DEPTH_BAND.1 - DEPTH_BAND.0),
            sun_texel,
            sun_depth,
            fog_color,
            fog_range,
            irradiance: sky.map_or([Vec4::ZERO; 9], |sky| sky.irradiance),
        };

        pipeline.prepare(encoder, &view, sun.shadows, sun.shadow_map_size);
    }

    pub fn draw(pass: &mut RenderPass) {
        if SceneManager::no_scene() {
            return;
        }

        let area = UIManager::render_area();
        pass.set_viewport(0.0, 0.0, area.width, area.height, DEPTH_BAND.0, DEPTH_BAND.1);

        let scene = SceneManager::scene();
        MESH.get_mut().draw(
            pass,
            scene.sky.as_ref(),
            scene.sky.is_some() || scene.fog.is_some(),
        );

        set_viewport(pass, UIManager::window_resolution());
    }
}
