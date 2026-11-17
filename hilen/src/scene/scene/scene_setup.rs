use crate::scene::Scene;

pub trait SceneSetup {
    fn setup(&mut self);

    /// Called once a frame before its physics steps, `dt` is the whole
    /// frame in seconds. Work that does not need every step goes here,
    /// like streaming the world or an aim cast. In `update` it ran once
    /// per step, 4 times a frame in a scene with physics.
    fn frame(&mut self, dt: f32);

    /// Called once per physics step, `dt` is that step in seconds. A scene
    /// with physics takes `SceneBase::PHYSICS_SUBSTEPS` steps a frame, so
    /// only what has to move with the physics belongs here, see `frame`.
    fn update(&mut self, dt: f32);

    /// Called right after each physics step, once the player and the
    /// camera moved, `dt` is the same step. What rides with the player,
    /// its body or its vehicle, is placed here, so it is drawn from the
    /// same step as the camera. Placed in `update` it lags 1 step of
    /// travel behind, 25 m at 6 km/s.
    fn after_physics(&mut self, dt: f32);

    fn needs_physics(&self) -> bool;

    /// Whether the scene uses the arrow keys and Enter itself, a game. The
    /// key focus is off while such a scene runs. A scene that only draws
    /// behind the views answers false, the keys and a remote then drive
    /// the views over it.
    fn takes_keys(&self) -> bool;
}

impl<T: Scene + 'static> SceneSetup for T {
    default fn setup(&mut self) {}

    default fn frame(&mut self, _: f32) {}

    default fn update(&mut self, _: f32) {}

    default fn after_physics(&mut self, _: f32) {}

    default fn needs_physics(&self) -> bool {
        false
    }

    default fn takes_keys(&self) -> bool {
        true
    }
}

pub trait SceneInternal {
    fn __internal_takes_keys(&self) -> bool;
    fn __internal_setup(&self);
    fn __internal_update(&self, frame_time: f32);
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::{
        deps::{hreads::set_current_thread_as_main, refs::Own},
        scene::{SceneBase, SceneInternal, SceneManager, SceneSetup, scene},
    };

    #[scene]
    #[derive(Default)]
    struct Counted {
        frames:      u32,
        frame_time:  f32,
        steps:       u32,
        after_steps: u32,
    }

    impl SceneSetup for Counted {
        fn needs_physics(&self) -> bool {
            true
        }

        fn frame(&mut self, dt: f32) {
            self.frames += 1;
            self.frame_time = dt;
        }

        fn update(&mut self, _: f32) {
            self.steps += 1;
        }

        fn after_physics(&mut self, _: f32) {
            self.after_steps += 1;
        }
    }

    // Work in `update` ran once per physics step, 4 times a frame, and a
    // game that streamed its world there paid for it 4 times.
    #[test]
    #[serial]
    fn a_frame_runs_once_and_update_once_per_step() {
        set_current_thread_as_main();
        let scene = Own::new(Counted::default());
        scene.__internal_setup();
        scene.__internal_update(1.0 / 60.0);

        let steps = u32::try_from(SceneBase::PHYSICS_SUBSTEPS).unwrap();
        assert_eq!(scene.frames, 1);
        assert!((scene.frame_time - 1.0 / 60.0).abs() < 1e-9);
        assert_eq!(scene.steps, steps);
        assert_eq!(scene.after_steps, steps);
    }

    // A game under its pause menu went on with its world, the pause was
    // only the test harness's own.
    #[test]
    #[serial]
    fn a_paused_scene_gets_no_frame_and_no_step() {
        set_current_thread_as_main();
        let scene = SceneManager::set_scene(Counted::default());

        SceneManager::set_paused(true);
        SceneManager::update();
        assert!(SceneManager::is_paused());
        assert_eq!((scene.frames, scene.steps), (0, 0));

        SceneManager::set_paused(false);
        SceneManager::update();
        assert_eq!(scene.frames, 1);

        SceneManager::set_paused(true);
        let scene = SceneManager::set_scene(Counted::default());
        assert!(!SceneManager::is_paused(), "a new scene starts running");
        SceneManager::update();
        assert_eq!(scene.frames, 1);
        SceneManager::stop_scene();
    }
}
