use anyhow::Result;
use ui_proc::view;

use crate::{
    SCENE_TESTS,
    deps::{hreads::from_main, refs::Weak},
    gm::flat::Point,
    scene::{Scene, SceneManager},
    ui::{Setup, UIEvents, ViewTouch},
    ui_test::{UITest, UITestEntry, get_test_name},
};

/// Radians of orbit or look per point of drag and the zoom per point of
/// wheel, the feel of the demo page.
const ORBIT_SPEED: f32 = 0.008;
const LOOK_SPEED: f32 = 0.004;
const ZOOM_SPEED: f32 = 0.002;

/// Implemented by `#[scene]` for every scene it can name concretely, so
/// a test on a generic scene is a compile error instead of one that
/// never runs. Same reason as `Registrable` for views.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is a generic scene, so it cannot be a scene test",
    label = "generic scene",
    note = "a test is registered by a ctor, and a ctor cannot name a generic type, so this test \
            would compile and then never run",
    note = "wrap it in a plain non generic scene and put the `impl SceneTest` on that instead"
)]
pub trait SceneRegistrable {}

/// A scene test. `impl SceneTest for X` on a `#[scene]` struct is the
/// whole declaration, the macro registers it. The runner installs an
/// empty root on the test canvas, starts the scene, hands it over and
/// stops it afterwards.
pub trait SceneTest: Scene + SceneRegistrable + Default {
    fn perform_test(scene: Weak<Self>) -> Result<()>;

    /// Screen pixels the test draws in, see `ViewTest::canvas`.
    fn canvas() -> (u32, u32) {
        (600, 600)
    }
}

/// The root a scene test runs under. The scene draws beneath the UI, so
/// the root only pins the canvas.
#[view]
pub struct SceneTestView {
    last_touch: Point,
}

impl SceneTestView {
    /// Drag anywhere to turn the camera around its target and the wheel
    /// zooms, or with a player in the scene the drag turns its head and
    /// the keys walk it. Presentation only, a test drives the camera
    /// itself.
    fn enable_orbit(mut self: Weak<Self>) {
        self.enable_touch();
        UIEvents::on_scroll().val(self, |delta| {
            let mut scene = SceneManager::scene_weak();
            if scene.player.is_none() {
                scene.camera.zoom(1.0 - delta.y * ZOOM_SPEED);
            }
        });
        self.touch().began.val(move |touch| self.last_touch = touch.position);
        self.touch().moved.val(move |touch| {
            let dx = touch.position.x - self.last_touch.x;
            let dy = touch.position.y - self.last_touch.y;
            self.last_touch = touch.position;
            let mut scene = SceneManager::scene_weak();
            match scene.player.as_mut() {
                Some(player) => player.look(dx * LOOK_SPEED, -dy * LOOK_SPEED),
                None => scene.camera.orbit(-dx * ORBIT_SPEED, dy * ORBIT_SPEED),
            }
        });
    }
}

/// Lets the `scene` macro ask a type whether it is a test, like
/// `MaybeLevelTest` does for levels.
pub trait MaybeSceneTest {
    fn __scene_test() -> Option<fn() -> Result<()>>;
    fn __scene_present() -> Option<fn()>;
}

impl<T: Scene> MaybeSceneTest for T {
    default fn __scene_test() -> Option<fn() -> Result<()>> {
        None
    }

    default fn __scene_present() -> Option<fn()> {
        None
    }
}

impl<T: Scene + SceneTest + 'static> MaybeSceneTest for T {
    fn __scene_test() -> Option<fn() -> Result<()>> {
        Some(|| {
            let (width, height) = T::canvas();

            UITest::set(SceneTestView::new(), width, height, true, get_test_name::<T>());

            let scene = from_main(|| SceneManager::set_scene(T::default()));

            let result = T::perform_test(scene);

            from_main(SceneManager::stop_scene);

            result
        })
    }

    fn __scene_present() -> Option<fn()> {
        Some(|| {
            let root = SceneTestView::new();
            let view = root.weak();
            UITest::present_root(root);
            from_main(move || {
                SceneManager::set_scene(T::default());
                view.enable_orbit();
            });
        })
    }
}

/// Registers `T` if, and only if, it implements [`SceneTest`]. Called
/// from the ctor `#[scene]` puts on every scene.
pub fn register_if_scene_test<T: Scene + 'static>(file: &'static str) {
    let (Some(run), Some(present)) = (
        <T as MaybeSceneTest>::__scene_test(),
        <T as MaybeSceneTest>::__scene_present(),
    ) else {
        return;
    };

    let name = get_test_name::<T>();

    assert!(
        SCENE_TESTS
            .lock()
            .insert(name.clone(), UITestEntry { run, present, file })
            .is_none(),
        "Duplicate scene test: {name}. The registry keys on the type name alone.",
    );
}
