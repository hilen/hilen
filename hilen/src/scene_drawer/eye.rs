use wgpu::{CommandEncoderDescriptor, TextureView};

use super::{PictureTargets, Route, SceneDrawer, mesh_for, targets_of};
use crate::{
    deps::refs::main_lock::MainLock,
    gm::flat::Size,
    scene::{Camera, SceneManager},
    window::Window,
};

/// The depth of an eye of a headset, both eyes have the same size.
static EYE_TARGETS: MainLock<Option<PictureTargets>> = MainLock::new();

impl SceneDrawer {
    /// Draws the running scene through the `camera` of 1 eye into `target`,
    /// an image of the headset, and submits it. 1 submit per eye, the
    /// camera uniform of the mesh pipeline is 1 buffer that every write of
    /// a submit lands in before any of its draws. 1 sample, the image is
    /// drawn straight with no resolve into it.
    pub(crate) fn draw_eye(camera: &Camera, target: &TextureView, size: Size<u32>) {
        let scene = SceneManager::scene();
        let targets = targets_of(EYE_TARGETS.get_mut(), size, 1);
        let mut encoder = Window::device().create_command_encoder(&CommandEncoderDescriptor {
            label: Some("xr_eye"),
        });
        Self::draw_into(
            scene,
            camera,
            target,
            size,
            targets,
            Route::Straight,
            &mut encoder,
        );
        crate::render::upload_pending();
        Window::queue().submit([encoder.finish()]);
        mesh_for(1).forget_frame();
    }
}
