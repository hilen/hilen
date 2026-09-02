use wgpu::RenderPass;

use crate::{
    deps::refs::main_lock::MainLock,
    gm::volume::Vec3,
    render::{MeshPipeline, SceneView, data::MeshInstance},
    scene::SceneManager,
    ui::{UIManager, ui_drawer::set_viewport},
};

static MESH: MainLock<MeshPipeline> = MainLock::new();

/// The scene owns this band of the frame's shared depth buffer. The UI
/// draws at 0.5 and closer, a level sprite at 0.85, so the UI stays in
/// front of any scene and a level can share the frame. The band costs
/// about one and a half bits of depth precision, the near plane of the
/// camera matters far more.
const DEPTH_BAND: (f32, f32) = (0.6, 1.0);

/// One fixed sun until the light list lands. Comes from behind the
/// default camera's right shoulder so every face of a box is shaded
/// differently.
const SUN_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);
const AMBIENT: f32 = 0.25;

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

        for node in scene.nodes() {
            if node.mesh.is_ok() {
                mesh.add(node.mesh, MeshInstance::new(node.model_matrix(), *node.color()));
            }
        }

        mesh.draw(
            pass,
            SceneView {
                view_proj: scene.camera.view_projection(area.width / area.height),
                light_dir: SUN_DIRECTION.normalize(),
                ambient:   AMBIENT,
            },
        );

        set_viewport(pass, UIManager::window_resolution());
    }
}
