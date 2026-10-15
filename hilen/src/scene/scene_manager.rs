use std::ops::{Deref, DerefMut};

use educe::Educe;
use rapier3d::{
    dynamics::{RigidBody, RigidBodyHandle},
    prelude::{Collider, ColliderHandle},
};

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{
        color::Color,
        flat::{Point, Size},
    },
    scene::{Scene, SceneView, scene::ScenePhysics},
    scene_drawer::SceneDrawer,
    ui::{UIManager, ViewFrame},
    window::image::Image,
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

    /// The view the scene draws in, see `SceneView`. None draws it over
    /// the whole root area.
    host: Option<Weak<SceneView>>,

    /// Holds the scene's time, see `set_paused`.
    paused: bool,

    /// Steps a stepped scene test may still take, see `set_stepped`.
    /// `None` runs free, one step per turn of the main loop.
    #[cfg(feature = "scene-tests")]
    steps_left: Option<u32>,
}

impl SceneManager {
    pub(crate) fn update() {
        // A scene lives as long as the view it draws in.
        if SELF.host.is_some_and(|host| !host.is_ok()) {
            Self::stop_scene();
        }

        if Self::no_scene() || SELF.paused {
            return;
        }

        #[cfg(feature = "scene-tests")]
        match &mut SELF.get_mut().steps_left {
            Some(0) => return,
            Some(left) => *left -= 1,
            None => {}
        }

        Self::scene().__internal_update(*Self::update_interval());
    }
}

impl SceneManager {
    /// Runs `scene` over the whole root area, behind every view. A scene
    /// that ran before, here or in a `SceneView`, stops.
    pub fn set_scene<T: Scene + 'static>(scene: T) -> Weak<T> {
        Self::install(scene, None)
    }

    /// Runs `scene` inside `host`, see `SceneView::set_scene`.
    pub(crate) fn set_scene_in<T: Scene + 'static>(scene: T, host: Weak<SceneView>) -> Weak<T> {
        Self::install(scene, Some(host))
    }

    fn install<T: Scene + 'static>(scene: T, host: Option<Weak<SceneView>>) -> Weak<T> {
        let s = SELF.get_mut();
        let scene = Own::new(scene);
        let weak = scene.weak();
        // The host is known before the setup, which may ask for the area.
        s.host = host;
        s.scene = Some(scene);
        s.scene.as_ref().unwrap().__internal_setup();
        weak
    }

    /// The `SceneView` the running scene draws in, none for a scene over
    /// the whole root area. The view can be gone already, then the scene
    /// draws nowhere and stops at the next update.
    pub fn host() -> Option<Weak<SceneView>> {
        SELF.host
    }

    /// The pixels the running scene draws in, what its rays and screen
    /// points are measured in.
    pub(crate) fn area() -> Size {
        match SELF.host {
            Some(host) if host.is_ok() => host.pixel_area(),
            _ => UIManager::render_area(),
        }
    }

    /// Where the area of the running scene starts, in points of the root
    /// view.
    pub(crate) fn origin() -> Point {
        match SELF.host {
            Some(host) if host.is_ok() => host.absolute_frame().origin,
            _ => Point::default(),
        }
    }

    /// Draws `scene` once into an image of `size` pixels and returns it, a
    /// picture of a model for a grid or a list. The scene is set up, drawn
    /// with its own camera, sun, lights, sky and nodes, and dropped. The
    /// running scene and the frame on screen are not touched, and none has
    /// to run. Where nothing is drawn the picture shows `background`,
    /// `CLEAR` leaves it see through, and a sky or fog covers it. The image
    /// is managed under `name` and the size, the same pair draws into the
    /// same image again. A picture has no time, so its scene has no physics,
    /// it is made of `Prop` nodes.
    pub fn picture<T: Scene + 'static>(
        scene: T,
        name: &str,
        size: impl Into<Size<u32>>,
        background: impl Into<Color>,
    ) -> Weak<Image> {
        let scene = Own::new(scene);
        scene.__internal_setup();
        assert!(
            !scene.has_physics(),
            "A scene picture has no physics. Build its scene of Prop nodes and leave needs_physics off."
        );
        SceneDrawer::picture(&*scene, name, size.into(), background.into())
    }

    pub fn stop_scene() {
        let s = SELF.get_mut();
        s.scene = None;
        s.host = None;
        #[cfg(feature = "scene-tests")]
        {
            s.steps_left = None;
        }
    }

    /// Stops the scene's time, the physics and every playing clip, and
    /// keeps drawing it. The test harness holds a scene this way while
    /// a human looks at its probes, so the picture under them stands
    /// still.
    pub(crate) fn set_paused(paused: bool) {
        SELF.get_mut().paused = paused;
    }

    /// Stepped, the scene's time stands still until `grant_steps` hands it
    /// steps. The main loop turns as often as the machine lets it, so a
    /// free running scene takes another number of steps between two waits
    /// of a test on every lane.
    #[cfg(feature = "scene-tests")]
    pub(crate) fn set_stepped(stepped: bool) {
        SELF.get_mut().steps_left = stepped.then_some(0);
    }

    #[cfg(feature = "scene-tests")]
    pub(crate) fn grant_steps(steps: u32) {
        let left =
            SELF.get_mut().steps_left.as_mut().expect(
                "step_scene on a scene that runs free. Return true from SceneTest::stepped to step it.",
            );
        *left += steps;
    }

    #[cfg(feature = "scene-tests")]
    pub(crate) fn steps_left() -> u32 {
        SELF.steps_left.unwrap_or(0)
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

    /// A scene runs and uses the arrow keys itself, see
    /// `SceneSetup::takes_keys`.
    pub(crate) fn takes_keys() -> bool {
        SELF.scene.as_ref().is_some_and(|scene| scene.__internal_takes_keys())
    }

    pub(crate) fn update_interval() -> &'static mut f32 {
        &mut SELF.get_mut().update_interval
    }
}
