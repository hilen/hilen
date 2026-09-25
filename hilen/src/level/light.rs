use crate::{
    deps::refs::{Own, Weak},
    gm::{
        ToF32,
        color::{Color, WHITE},
        flat::Point,
    },
    level::LevelBase,
};

/// A point light in a level. Every sprite and tile map is lit by the
/// level's `ambient_light` plus every light in reach. At the center a
/// light adds `color` times `intensity`, the add fades to nothing at
/// `radius`, and `falloff` bends that fade, 1 is linear and a larger one
/// keeps the light close to its center. The sum is capped at white.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub position:  Point,
    pub color:     Color,
    pub radius:    f32,
    pub intensity: f32,
    pub falloff:   f32,
}

impl Light {
    pub fn new(position: impl Into<Point>, radius: impl ToF32) -> Self {
        Self {
            position:  position.into(),
            color:     WHITE,
            radius:    radius.to_f32(),
            intensity: 1.0,
            falloff:   2.0,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_intensity(mut self, intensity: impl ToF32) -> Self {
        self.intensity = intensity.to_f32();
        self
    }

    pub fn with_falloff(mut self, falloff: impl ToF32) -> Self {
        self.falloff = falloff.to_f32();
        self
    }
}

impl LevelBase {
    /// Adds a light, the handle moves or changes it like a sprite.
    pub fn add_light(&mut self, light: Light) -> Weak<Light> {
        let light = Own::new(light);
        let weak = light.weak();
        self.lights.push(light);
        weak
    }

    pub fn remove_light(&mut self, light: Weak<Light>) {
        self.lights.retain(|own| own.raw() != light.raw());
    }

    pub fn lights(&self) -> &[Own<Light>] {
        &self.lights
    }
}
