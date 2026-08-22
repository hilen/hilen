use crate::{gm::volume::GyroData, level::Level};

pub trait LevelSetup {
    fn setup(&mut self);

    fn update(&mut self);

    fn on_key_pressed(&mut self, _: char);

    fn on_gyro_changed(&mut self, _: GyroData);

    fn needs_physics(&self) -> bool;
}

impl<T: Level + 'static> LevelSetup for T {
    default fn setup(&mut self) {}

    default fn update(&mut self) {}

    default fn on_key_pressed(&mut self, _: char) {}

    default fn on_gyro_changed(&mut self, _: GyroData) {}

    default fn needs_physics(&self) -> bool {
        false
    }
}

pub trait LevelInternal {
    fn __internal_setup(&self);
    fn __internal_update(&self, frame_time: f32);
}
