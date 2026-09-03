use crate::scene::Scene;

pub trait SceneSetup {
    fn setup(&mut self);

    /// Called once per physics step, `dt` is that step in seconds.
    fn update(&mut self, dt: f32);

    fn needs_physics(&self) -> bool;
}

impl<T: Scene + 'static> SceneSetup for T {
    default fn setup(&mut self) {}

    default fn update(&mut self, _: f32) {}

    default fn needs_physics(&self) -> bool {
        false
    }
}

pub trait SceneInternal {
    fn __internal_setup(&self);
    fn __internal_update(&self, frame_time: f32);
}
