use crate::{
    gm::{
        color::{Color, WHITE},
        volume::{Vec3, Vec4},
    },
    render::{MeshPipeline, data::MeshLight},
};

/// The most lights one node is drawn with.
pub(crate) const MAX_LIGHTS: usize = 8;

/// The one directional light of a scene. `direction` is where the light
/// travels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    pub direction:       Vec3,
    pub color:           Color,
    /// The brightness of a white matte surface facing the sun, 1 is full
    /// white.
    pub intensity:       f32,
    /// Whether the sun casts shadows, cascaded shadow maps drawn in
    /// passes before the frame, see `scene::shadow`. Off by default,
    /// every pass draws every opaque node again.
    pub shadows:         bool,
    /// How far from the camera the shadows reach. The cascades share
    /// this range, so a shorter one gives every map finer texels, and
    /// past it the sun shines through. Infinite by default, then the
    /// whole scene casts, which on a big level makes coarse shadows.
    pub shadow_distance: f32,
    /// Texels along each side of every cascade's map, a power of two.
    /// Changing it at runtime remakes the maps on the next frame.
    pub shadow_map_size: u32,
}

impl Default for Sun {
    /// From behind the default camera's right shoulder, so every face
    /// of a box is shaded differently. With the default ambient a white
    /// surface facing it shows full white.
    fn default() -> Self {
        Self {
            direction:       Vec3::new(-0.4, -1.0, -0.6),
            color:           WHITE,
            intensity:       0.75,
            shadows:         false,
            shadow_distance: f32::INFINITY,
            shadow_map_size: MeshPipeline::SHADOW_MAP_SIZE,
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
