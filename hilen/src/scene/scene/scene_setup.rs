use crate::scene::Scene;

pub trait SceneSetup {
    fn setup(&mut self);

    /// Called once per physics step, `dt` is that step in seconds.
    fn update(&mut self, dt: f32);

    fn needs_physics(&self) -> bool;

    /// Whether the scene uses the arrow keys and Enter itself, a game. The
    /// key focus is off while such a scene runs. A scene that only draws
    /// behind the views answers false, the keys and a remote then drive
    /// the views over it.
    fn takes_keys(&self) -> bool;
}

impl<T: Scene + 'static> SceneSetup for T {
    default fn setup(&mut self) {}

    default fn update(&mut self, _: f32) {}

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
