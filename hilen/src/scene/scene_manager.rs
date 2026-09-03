use std::ops::{Deref, DerefMut};

use educe::Educe;
use rapier3d::{
    dynamics::{RigidBody, RigidBodyHandle},
    prelude::{Collider, ColliderHandle},
};

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    scene::{Scene, scene::ScenePhysics},
};

static SELF: MainLock<SceneManager> = MainLock::new();

/// The one running scene, the twin of `LevelManager`. The camera lives
/// on the scene itself, so there is no scale or camera position here.
#[derive(Educe)]
#[educe(Default)]
pub struct SceneManager {
    #[educe(Default = 1.0 / 60.0)]
    update_interval: f32,

    scene: Option<Own<dyn Scene>>,

    /// Holds the scene's time, see `set_paused`.
    paused: bool,
}

impl SceneManager {
    pub(crate) fn update() {
        if Self::no_scene() || SELF.paused {
            return;
        }

        Self::scene().__internal_update(*Self::update_interval());
    }
}

impl SceneManager {
    pub fn set_scene<T: Scene + 'static>(scene: T) -> Weak<T> {
        let s = SELF.get_mut();
        let scene = Own::new(scene);
        let weak = scene.weak();
        s.scene = Some(scene);
        s.scene.as_ref().unwrap().__internal_setup();
        weak
    }

    pub fn stop_scene() {
        SELF.get_mut().scene = None;
    }

    /// Stops the scene's time, the physics and every playing clip, and
    /// keeps drawing it. The test harness holds a scene this way while
    /// a human looks at its probes, so the picture under them stands
    /// still.
    pub(crate) fn set_paused(paused: bool) {
        SELF.get_mut().paused = paused;
    }

    pub(crate) fn scene() -> &'static dyn Scene {
        SELF.scene.as_ref().expect("No Scene").deref()
    }

    pub fn scene_weak() -> Weak<dyn Scene> {
        SELF.scene.as_ref().expect("No Scene").weak()
    }

    pub(crate) unsafe fn scene_unchecked() -> &'static mut dyn Scene {
        unsafe { SELF.get_unchecked().scene.as_mut().expect("No Scene").deref_mut() }
    }

    pub(crate) fn physics() -> &'static mut ScenePhysics {
        unsafe {
            Self::scene_unchecked()
                .physics
                .as_mut()
                .expect("This scene has no physics enabled. Override SceneSetup::needs_physics to enable.")
        }
    }

    pub fn downcast_scene<T: Scene + 'static>() -> Weak<T> {
        Self::scene_weak().downcast::<T>().unwrap()
    }

    pub(crate) fn get_rigid_body(handle: RigidBodyHandle) -> &'static RigidBody {
        &SceneManager::physics().sets.rigid_bodies[handle]
    }

    pub(crate) fn get_collider(handle: ColliderHandle) -> &'static Collider {
        &SceneManager::physics().sets.colliders[handle]
    }

    pub(crate) fn no_scene() -> bool {
        SELF.scene.is_none()
    }

    pub(crate) fn update_interval() -> &'static mut f32 {
        &mut SELF.get_mut().update_interval
    }
}
