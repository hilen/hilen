use crate::{gm::volume::GyroData, level::Level};

pub trait LevelSetup {
    fn setup(&mut self);

    /// Called once per physics step, `dt` is that step in seconds.
    fn update(&mut self, dt: f32);

    fn on_key_pressed(&mut self, _: char);

    fn on_gyro_changed(&mut self, _: GyroData);

    fn needs_physics(&self) -> bool;

    /// Whether the level uses the arrow keys and Enter itself, a game. The
    /// key focus is off while such a level runs. A level that only draws
    /// behind the views answers false, the keys and a remote then drive
    /// the views over it.
    fn takes_keys(&self) -> bool;
}

impl<T: Level + 'static> LevelSetup for T {
    default fn setup(&mut self) {}

    default fn update(&mut self, _: f32) {}

    default fn on_key_pressed(&mut self, _: char) {}

    default fn on_gyro_changed(&mut self, _: GyroData) {}

    default fn needs_physics(&self) -> bool {
        false
    }

    default fn takes_keys(&self) -> bool {
        true
    }
}

pub trait LevelInternal {
    fn __internal_takes_keys(&self) -> bool;
    fn __internal_setup(&self);
    fn __internal_update(&self, frame_time: f32);
}
