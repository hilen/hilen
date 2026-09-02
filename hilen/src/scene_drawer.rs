use wgpu::RenderPass;

use crate::{
    deps::refs::main_lock::MainLock,
    gm::volume::Vec4,
    render::{MeshKey, MeshPipeline, SceneView, data::MeshInstance},
    scene::{SceneManager, pick_lights},
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

impl SceneDrawer {
    pub(crate) fn update() {
        SceneManager::update();

        // A running scene animates every frame like a level does.
        if !SceneManager::no_scene() {
            crate::window::request_frame();
        }
    }

    pub fn draw(pass: &mut RenderPass) {
        if SceneManager::no_scene() {
            return;
        }

        let area = UIManager::render_area();
        pass.set_viewport(0.0, 0.0, area.width, area.height, DEPTH_BAND.0, DEPTH_BAND.1);

        let scene = SceneManager::scene();
        let mesh = MESH.get_mut();
        let camera = scene.camera;

        for light in &scene.lights {
            mesh.add_light(light.mesh_light());
        }

        // Translucent nodes blend over what is behind them, so they draw
        // after every opaque node, the farthest first.
        let mut translucent = vec![];

        for node in scene.nodes() {
            if !node.mesh.is_ok() {
                continue;
            }
            let position = node.position();
            let reach = node.shape().half_extents().length();
            let lights = pick_lights(position, reach, &scene.lights);
            let material = node.material;
            let key = MeshKey {
                mesh:       node.mesh,
                texture:    material.texture,
                normal_map: material.normal_map,
            };
            let instance = MeshInstance::new(node.model_matrix(), material, lights);
            if material.color.a < 1.0 {
                translucent.push((camera.position.distance_squared(position), key, instance));
            } else {
                mesh.add(key, instance);
            }
        }

        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, key, instance) in translucent {
            mesh.add_transparent(key, instance);
        }

        let sun = scene.sun;
        let sun_color = sun.color.linear();
        let ambient = scene.ambient.linear();
        let sky = scene.sky.as_ref();
        let view_proj = camera.view_projection(area.width / area.height);

        mesh.draw(
            pass,
            &SceneView {
                view_proj,
                inv_view_proj: view_proj.inverse(),
                camera_pos: camera.position.extend(0.0),
                sun_dir: sun.direction.normalize_or_zero().extend(0.0),
                sun_color: Vec4::new(
                    sun_color.r * sun.intensity,
                    sun_color.g * sun.intensity,
                    sun_color.b * sun.intensity,
                    0.0,
                ),
                ambient: Vec4::new(
                    ambient.r,
                    ambient.g,
                    ambient.b,
                    f32::from(u8::from(sky.is_some())),
                ),
                viewport: Vec4::new(area.width, area.height, DEPTH_BAND.0, DEPTH_BAND.1 - DEPTH_BAND.0),
                irradiance: sky.map_or([Vec4::ZERO; 9], |sky| sky.irradiance),
            },
            sky,
        );

        set_viewport(pass, UIManager::window_resolution());
    }
}
