use glam::camera::rh::{proj::directx::orthographic, view::look_at_mat4};

use crate::{
    gm::{
        LossyConvert,
        color::{Color, WHITE},
        volume::{Bounds, Mat4, Vec3, Vec4},
    },
    render::data::MeshLight,
};

/// The most lights one node is drawn with.
pub(crate) const MAX_LIGHTS: usize = 8;

/// The one directional light of a scene. `direction` is where the light
/// travels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    pub direction: Vec3,
    pub color:     Color,
    /// The brightness of a white matte surface facing the sun, 1 is full
    /// white.
    pub intensity: f32,
    /// Whether the sun casts shadows, one shadow map over the whole
    /// scene drawn in a pass before the frame. Off by default, the pass
    /// draws every opaque node a second time.
    pub shadows:   bool,
}

impl Sun {
    /// The light's view projection over the sphere around `bounds`,
    /// and the world size of one texel of a `map_size` wide shadow map
    /// at that fit. Orthographic, the sun is infinitely far.
    pub(crate) fn shadow_view(&self, bounds: Bounds, map_size: u32) -> (Mat4, f32) {
        let center = bounds.center();
        let radius = bounds.half_extents().length().max(1.0);
        let direction = self.direction.normalize_or(Vec3::NEG_Y);
        // Straight down the world's up lines up with the view and the
        // matrix degenerates.
        let up = if direction.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let view = look_at_mat4(center - direction * radius * 2.0, center, up);
        let projection = orthographic(-radius, radius, -radius, radius, radius, radius * 3.0);
        (projection * view, 2.0 * radius / map_size.lossy_convert())
    }
}

impl Default for Sun {
    /// From behind the default camera's right shoulder, so every face
    /// of a box is shaded differently. With the default ambient a white
    /// surface facing it shows full white.
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.4, -1.0, -0.6),
            color:     WHITE,
            intensity: 0.75,
            shadows:   false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightKind {
    Point,
    /// A cone along `direction`, full inside the `inner` half angle and
    /// gone past `outer`, both in radians.
    Spot {
        direction: Vec3,
        inner:     f32,
        outer:     f32,
    },
}

/// A point or spot light. A scene holds any number of them and every
/// node is drawn with the nearest `MAX_LIGHTS` of those in reach of it,
/// see `pick_lights`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub position:  Vec3,
    pub color:     Color,
    /// The brightness of a white matte surface facing the light one unit
    /// away. It falls off with the square of the distance.
    pub intensity: f32,
    /// Nothing past this distance is lit, the falloff is pulled to zero
    /// there.
    pub range:     f32,
    pub kind:      LightKind,
}

impl Light {
    pub fn point(position: impl Into<Vec3>) -> Self {
        Self {
            position:  position.into(),
            color:     WHITE,
            intensity: 1.0,
            range:     10.0,
            kind:      LightKind::Point,
        }
    }

    /// A cone from `position` along `direction`, `angle` the half angle
    /// of the cone in radians. The edge softens over its outer fifth.
    pub fn spot(position: impl Into<Vec3>, direction: impl Into<Vec3>, angle: f32) -> Self {
        Self {
            kind: LightKind::Spot {
                direction: direction.into(),
                inner:     angle * 0.8,
                outer:     angle,
            },
            ..Self::point(position)
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    /// The light as the mesh shader reads it. The cone term of the
    /// shader is `saturate(cos * scale + offset)` squared, so a point
    /// light gets scale 0 and offset 1 and the term is always one.
    pub(crate) fn mesh_light(&self) -> MeshLight {
        let falloff = 1.0 / (self.range * self.range).max(1e-4);

        let (direction, scale, offset) = match self.kind {
            LightKind::Point => (Vec3::ZERO, 0.0, 1.0),
            LightKind::Spot {
                direction,
                inner,
                outer,
            } => {
                let cos_inner = inner.cos();
                let cos_outer = outer.cos();
                let scale = 1.0 / (cos_inner - cos_outer).max(1e-4);
                (direction.normalize_or_zero(), scale, -cos_outer * scale)
            }
        };

        let radiance = self.color.linear();

        MeshLight {
            position:  self.position.extend(falloff),
            direction: direction.extend(scale),
            color:     Vec4::new(
                radiance.r * self.intensity,
                radiance.g * self.intensity,
                radiance.b * self.intensity,
                offset,
            ),
        }
    }
}

/// The lights one node is drawn with, indices into the scene's list
/// packed two to a word the way the instance buffer carries them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LightPick {
    pub packed: [u32; 4],
    pub count:  u32,
}

/// The nearest `MAX_LIGHTS` lights whose range reaches the sphere of
/// `radius` around `position`, nearest first, ties by list order.
pub(crate) fn pick_lights(position: Vec3, radius: f32, lights: &[Light]) -> LightPick {
    let mut nearest = [(f32::INFINITY, 0usize); MAX_LIGHTS];
    let mut count = 0;

    for (index, light) in lights.iter().enumerate() {
        let distance = light.position.distance(position);
        if distance - radius >= light.range {
            continue;
        }
        let slot = nearest[..count].partition_point(|near| near.0 <= distance);
        if slot >= MAX_LIGHTS {
            continue;
        }
        let end = count.min(MAX_LIGHTS - 1);
        nearest.copy_within(slot..end, slot + 1);
        nearest[slot] = (distance, index);
        count = (count + 1).min(MAX_LIGHTS);
    }

    let mut pick = LightPick::default();
    for (slot, (_, index)) in nearest[..count].iter().enumerate() {
        let index = u16::try_from(*index).expect("a scene holds at most 65536 point and spot lights");
        pick.packed[slot / 2] |= u32::from(index) << ((slot % 2) * 16);
    }
    pick.count = u32::try_from(count).expect("MAX_LIGHTS fits a u32");
    pick
}

#[cfg(test)]
mod test {
    use super::*;

    fn indices(pick: LightPick) -> Vec<u32> {
        (0..pick.count as usize)
            .map(|slot| (pick.packed[slot / 2] >> ((slot % 2) * 16)) & 0xffff)
            .collect()
    }

    // The node is drawn with the nearest lights first, so a ninth light
    // drops the farthest one and not the last one added.
    #[test]
    fn picks_the_nearest_eight() {
        let lights: Vec<Light> = (0..12u8)
            .rev()
            .map(|i| Light::point(Vec3::new(f32::from(i), 0.0, 0.0)).range(100.0))
            .collect();
        let pick = pick_lights(Vec3::ZERO, 0.0, &lights);
        assert_eq!(pick.count, 8);
        assert_eq!(indices(pick), vec![11, 10, 9, 8, 7, 6, 5, 4]);
    }

    // A light whose range ends short of the node's surface does nothing
    // to it, so it must not take one of the eight slots.
    #[test]
    fn out_of_reach_is_dropped_and_the_radius_counts() {
        let far = Light::point(Vec3::new(5.0, 0.0, 0.0)).range(3.0);
        let near = Light::point(Vec3::new(5.0, 0.0, 0.0)).range(4.5);
        let pick = pick_lights(Vec3::ZERO, 1.0, &[far, near]);
        assert_eq!(indices(pick), vec![1]);
    }

    #[test]
    fn no_lights_is_an_empty_pick() {
        assert_eq!(pick_lights(Vec3::ZERO, 1.0, &[]), LightPick::default());
    }

    // A point light must never be cut by the cone term, whatever the
    // angle, so its scale is zero and its offset one.
    #[test]
    fn point_light_has_no_cone() {
        let light = Light::point(Vec3::ZERO).range(2.0).mesh_light();
        assert!(light.direction.w.abs() < f32::EPSILON);
        assert!((light.color.w - 1.0).abs() < f32::EPSILON);
        assert!((light.position.w - 0.25).abs() < 1e-6);
    }

    #[test]
    fn spot_cone_is_full_inside_and_gone_outside() {
        let light = Light::spot(Vec3::ZERO, Vec3::NEG_Y, 0.5).mesh_light();
        let cone = |cos: f32| (cos * light.direction.w + light.color.w).clamp(0.0, 1.0);
        assert!((cone(0.4_f32.cos()) - 1.0).abs() < 1e-5);
        assert!(cone(0.5_f32.cos()) < 1e-5);
        assert!(cone(0.45_f32.cos()) > 0.0 && cone(0.45_f32.cos()) < 1.0);
    }
}

#[cfg(test)]
mod shadow_test {
    use super::*;

    // The whole sphere around the bounds must land inside the map, and
    // the far side must not fall off the depth range.
    #[test]
    fn the_shadow_view_holds_the_scene() {
        let sun = Sun {
            direction: Vec3::new(-0.4, -1.0, -0.6),
            ..Sun::default()
        };
        let bounds = Bounds {
            min: Vec3::new(-8.0, -1.0, -8.0),
            max: Vec3::new(8.0, 5.0, 8.0),
        };
        let (view_proj, texel) = sun.shadow_view(bounds, 1024);
        let radius = bounds.half_extents().length();
        assert!((texel - 2.0 * radius / 1024.0).abs() < 1e-5);
        for corner in [
            bounds.min,
            bounds.max,
            Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
            Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        ] {
            let clip = view_proj.project_point3(corner);
            assert!(
                clip.x.abs() <= 1.0 && clip.y.abs() <= 1.0,
                "{corner:?} lands at {clip:?}"
            );
            assert!((0.0..=1.0).contains(&clip.z), "{corner:?} depth {}", clip.z);
        }
    }
}
