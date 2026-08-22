use std::fmt::Display;

#[derive(Default, Debug, Copy, Clone)]
pub struct GyroData {
    pub pitch: f32,
    pub roll:  f32,
    pub yaw:   f32,
}

impl Display for GyroData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pitch: {}, Roll: {}, Yaw: {}", self.pitch, self.roll, self.yaw)
    }
}
