use std::array::from_fn;

use glam::camera::rh::{proj::directx::orthographic, view::look_at_mat4};

use crate::{
    gm::{
        LossyConvert,
        volume::{Bounds, Mat4, Vec3},
    },
    render::SHADOW_CASCADES,
    scene::{Camera, Sun},
};

/// One slice of the sun's shadow: the light's view projection into its
/// layer of the shadow map, the world size of one texel of it, and the
/// world length its depth range spans, what turns a world bias into a
/// depth one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShadowCascade {
    pub view_proj: Mat4,
    pub texel:     f32,
    pub depth:     f32,
}

/// How far the splits lean from even towards logarithmic. Even splits
/// spend the near map on empty air, logarithmic ones give the far half
/// of the view one map, halfway suits a room and a field alike.
const SPLIT_BLEND: f32 = 0.5;

/// The ratio between the rungs the ends of the range snap to.
const RANGE_STEP: f32 = 1.25;

#[derive(Clone, Copy, Debug)]
struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    /// Around `bounds`, at least a unit wide so an empty scene still
    /// gets a map.
    fn around(bounds: Bounds) -> Self {
        Self {
            center: bounds.center(),
            radius: bounds.half_extents().length().max(1.0),
        }
    }
}

/// The sun's cascades over `scene`, the box around every node: the
/// camera's view cut into `SHADOW_CASCADES` depth ranges, each with its
/// own map fit around the sphere of its slice, so the near slice gets
/// fine texels and the far one coarse ones. The range starts where the
/// view enters the scene and ends where it leaves it or at the sun's
/// `shadow_distance`, so a small scene seen from afar still gets every
/// map. A map never grows past the scene's own sphere, and its origin
/// snaps to a texel so a moving camera does not shimmer the shadow
/// edges.
pub(crate) fn sun_cascades(
    sun: &Sun,
    camera: &Camera,
    aspect: f32,
    scene: Bounds,
) -> [ShadowCascade; SHADOW_CASCADES] {
    let scene = Sphere::around(scene);
    let direction = sun.direction.normalize_or(Vec3::NEG_Y);
    let forward = (camera.target - camera.position).normalize_or(Vec3::NEG_Z);
    let (near, far) = shadow_range(sun, camera, forward, scene);
    let splits = split_depths(near, far);

    from_fn(|index| {
        let slice = slice_sphere(camera, aspect, forward, splits[index], splits[index + 1]);
        let sphere = if slice.radius >= scene.radius {
            scene
        } else {
            slice
        };
        cascade(sphere, scene, direction, sun.shadow_map_size)
    })
}

/// The view depths where the shadows start and end: where the view
/// enters the scene's sphere and where it leaves it, inside the
/// camera's own planes and the sun's reach. Both snap outwards to a
/// rung of a geometric ladder, so a walking camera moves them, and
/// with them the texel size of every map, now and then instead of
/// every frame, and the texel snapping of the maps holds in between.
fn shadow_range(sun: &Sun, camera: &Camera, forward: Vec3, scene: Sphere) -> (f32, f32) {
    let to_center = (scene.center - camera.position).dot(forward);
    let near = rung(camera.near.max(to_center - scene.radius).max(1e-3), false);
    let far = camera
        .far
        .min(to_center + scene.radius)
        .min(sun.shadow_distance)
        .max(near * RANGE_STEP);
    (near, rung(far, true))
}

fn rung(depth: f32, up: bool) -> f32 {
    let steps = depth.log(RANGE_STEP);
    let steps = if up { steps.ceil() } else { steps.floor() };
    RANGE_STEP.powf(steps)
}

/// The depths that bound every cascade, `near` first and `far` last,
/// the practical split between even and logarithmic.
fn split_depths(near: f32, far: f32) -> [f32; SHADOW_CASCADES + 1] {
    let count: f32 = SHADOW_CASCADES.lossy_convert();
    from_fn(|index| {
        let fraction: f32 = index.lossy_convert();
        let fraction = fraction / count;
        let even = near + (far - near) * fraction;
        let log = near * (far / near).powf(fraction);
        even + (log - even) * SPLIT_BLEND
    })
}

/// The sphere around the part of the view between depths `near` and
/// `far`, its center on the line of sight where the near corners and
/// the far corners are equally far, or at the far plane when that
/// point would lie past it.
fn slice_sphere(camera: &Camera, aspect: f32, forward: Vec3, near: f32, far: f32) -> Sphere {
    let tan = (camera.fov_y / 2.0).tan();
    let corner = tan * tan * (1.0 + aspect * aspect);
    let depth = ((near + far) * (1.0 + corner) / 2.0).min(far);
    Sphere {
        center: camera.position + forward * depth,
        radius: ((far - depth).powi(2) + far * far * corner).sqrt(),
    }
}

/// The map over `sphere`, orthographic, the sun is infinitely far. Its
/// depth range starts back at the scene's far side towards the sun,
/// since anything of the scene between the slice and the sun casts
/// into it.
fn cascade(sphere: Sphere, scene: Sphere, direction: Vec3, map_size: u32) -> ShadowCascade {
    let texel = 2.0 * sphere.radius / map_size.lossy_convert();
    let up = light_up(direction);
    let light_space = look_at_mat4(Vec3::ZERO, direction, up);

    let placed = light_space.transform_point3(sphere.center);
    let snapped = Vec3::new(
        (placed.x / texel).round() * texel,
        (placed.y / texel).round() * texel,
        placed.z,
    );
    let center = light_space.inverse().transform_point3(snapped);

    let reach = (center.distance(scene.center) + scene.radius).max(sphere.radius);
    let view = look_at_mat4(center - direction * reach, center, up);
    let radius = sphere.radius;
    let depth = reach + radius;
    let projection = orthographic(-radius, radius, -radius, radius, 0.0, depth);

    ShadowCascade {
        view_proj: projection * view,
        texel,
        depth,
    }
}

/// Straight down the world's up lines up with the view and the matrix
/// degenerates.
fn light_up(direction: Vec3) -> Vec3 {
    if direction.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const MAP_SIZE: u32 = 1024;

    fn sun() -> Sun {
        Sun {
            shadow_map_size: MAP_SIZE,
            ..Sun::default()
        }
    }

    fn field() -> Bounds {
        Bounds {
            min: Vec3::new(-40.0, 0.0, -40.0),
            max: Vec3::new(40.0, 6.0, 40.0),
        }
    }

    fn low_camera() -> Camera {
        Camera {
            position: Vec3::new(0.0, 2.0, 30.0),
            target: Vec3::new(0.0, 1.0, -30.0),
            ..Camera::default()
        }
    }

    /// Whether `point` lands in the map, the texel its origin snapped
    /// by allowed.
    fn inside_map(cascade: &ShadowCascade, point: Vec3) -> bool {
        let clip = cascade.view_proj.project_point3(point);
        let slack = 2.0 / f32::from(u16::try_from(MAP_SIZE).expect("test map size"));
        clip.x.abs() <= 1.0 + slack && clip.y.abs() <= 1.0 + slack && (0.0..=1.0).contains(&clip.z)
    }

    /// The four corners of the view at depth `depth`.
    fn view_corners(camera: &Camera, aspect: f32, depth: f32) -> [Vec3; 4] {
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward);
        let half_height = depth * (camera.fov_y / 2.0).tan();
        let half_width = half_height * aspect;
        let center = camera.position + forward * depth;
        [
            center + right * half_width + up * half_height,
            center - right * half_width + up * half_height,
            center + right * half_width - up * half_height,
            center - right * half_width - up * half_height,
        ]
    }

    // The whole point of the cascades: the near slice is drawn finer
    // than the far one.
    #[test]
    fn the_near_cascade_has_the_finest_texels() {
        let cascades = sun_cascades(&sun(), &low_camera(), 1.0, field());
        assert!(cascades[0].texel < cascades[1].texel);
        assert!(cascades[1].texel < cascades[2].texel);
    }

    // Every point of a slice of the view that is inside the scene lands
    // inside the map of its cascade, or a receiver there would go
    // unshadowed. Outside the scene there is nothing to shadow.
    #[test]
    fn every_slice_lands_in_its_map() {
        let camera = low_camera();
        let aspect = 1.5;
        let scene = Sphere::around(field());
        let forward = (camera.target - camera.position).normalize();
        let (near, far) = shadow_range(&sun(), &camera, forward, scene);
        let splits = split_depths(near, far);
        let cascades = sun_cascades(&sun(), &camera, aspect, field());
        let mut checked = 0;

        for (index, cascade) in cascades.iter().enumerate() {
            for depth in [splits[index], splits[index + 1]] {
                for corner in view_corners(&camera, aspect, depth) {
                    if corner.distance(scene.center) > scene.radius {
                        continue;
                    }
                    checked += 1;
                    assert!(
                        inside_map(cascade, corner),
                        "cascade {index} misses {corner:?} at depth {depth}"
                    );
                }
            }
        }
        assert!(checked >= 8, "only {checked} corners were inside the scene");
    }

    #[test]
    fn splits_climb_from_near_to_far() {
        let splits = split_depths(0.5, 100.0);
        assert!((splits[0] - 0.5).abs() < 1e-6);
        assert!((splits[SHADOW_CASCADES] - 100.0).abs() < 1e-4);
        for pair in splits.windows(2) {
            assert!(pair[0] < pair[1], "{splits:?}");
        }
        // Leaning logarithmic, the first slice is shorter than an even
        // share.
        assert!(splits[1] - splits[0] < 100.0 / 3.0);
    }

    // The range holds the scene however the ends snap, near rounds
    // down and far rounds up, and they only move when the camera
    // crosses a rung.
    #[test]
    fn the_range_snaps_outwards_to_a_rung() {
        let scene = Sphere::around(field());
        let camera = low_camera();
        let forward = (camera.target - camera.position).normalize();
        let (near, far) = shadow_range(&sun(), &camera, forward, scene);
        let to_center = (scene.center - camera.position).dot(forward);
        assert!(near <= camera.near);
        assert!(far >= to_center + scene.radius);
        assert!(far < (to_center + scene.radius) * RANGE_STEP);

        let mut moved = camera;
        moved.position.z -= 1.0;
        moved.target.z -= 1.0;
        assert_eq!(shadow_range(&sun(), &moved, forward, scene), (near, far));
    }

    // A small scene far from the camera is what the old single map
    // covered whole. Now the shadows start where the view enters the
    // scene, so every cascade lies on the scene and the whole scene
    // still lands in the last map.
    #[test]
    fn a_small_scene_seen_from_afar_gets_every_map() {
        let bounds = Bounds {
            min: Vec3::new(-2.0, 0.0, -2.0),
            max: Vec3::new(2.0, 3.0, 2.0),
        };
        let camera = Camera {
            position: Vec3::new(0.0, 5.0, 30.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
        let scene = Sphere::around(bounds);
        let forward = (camera.target - camera.position).normalize();
        let (near, far) = shadow_range(&sun(), &camera, forward, scene);
        assert!(near > 20.0, "shadows start at {near}, well before the scene");
        assert!(far < 40.0);

        let cascades = sun_cascades(&sun(), &camera, 1.0, bounds);
        let scene_texel = 2.0 * scene.radius / 1024.0;
        for cascade in &cascades {
            assert!(cascade.texel <= scene_texel + 1e-6);
        }
        for corner in [
            bounds.min,
            bounds.max,
            Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
            Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        ] {
            assert!(
                cascades.iter().any(|cascade| inside_map(cascade, corner)),
                "{corner:?} lands in no map"
            );
        }
    }

    // A finite reach cuts the range short of the scene, so the cascades
    // split a shorter range into finer maps, and the last map still
    // holds everything up to the reach.
    #[test]
    fn the_shadow_distance_caps_the_range_and_refines_the_maps() {
        let camera = low_camera();
        let near = Sun {
            shadow_distance: 30.0,
            ..sun()
        };
        let forward = (camera.target - camera.position).normalize();
        let (_, far) = shadow_range(&near, &camera, forward, Sphere::around(field()));
        assert!((30.0..40.0).contains(&far), "far {far}");

        let whole = sun_cascades(&sun(), &camera, 1.0, field());
        let capped = sun_cascades(&near, &camera, 1.0, field());
        for (whole, capped) in whole.iter().zip(&capped) {
            assert!(capped.texel <= whole.texel);
        }
        assert!(capped[2].texel < whole[2].texel * 0.6);
        let ahead = camera.position + forward * 29.0;
        assert!(inside_map(&capped[2], ahead));
    }

    // A post outside the near slice but between it and the sun throws
    // its shadow into the slice, so it must sit inside the near map's
    // depth range and not be clipped away.
    #[test]
    fn a_caster_towards_the_sun_stays_in_depth() {
        let sun = Sun {
            direction: Vec3::new(-1.0, -0.4, 0.0),
            ..sun()
        };
        let cascades = sun_cascades(&sun, &low_camera(), 1.0, field());
        // The far east edge of the field, the side the sun comes from.
        let caster = Vec3::new(40.0, 6.0, 25.0);
        let clip = cascades[0].view_proj.project_point3(caster);
        assert!(clip.z >= 0.0 && clip.z <= 1.0, "caster depth {}", clip.z);
    }

    // The camera moving inside one texel must not move the map, or
    // every shadow edge shimmers as it walks.
    #[test]
    fn the_map_moves_by_whole_texels() {
        let camera = low_camera();
        let mut moved = camera;
        let step = Vec3::new(0.0137, 0.0, -0.0071);
        moved.position += step;
        moved.target += step;

        let before = sun_cascades(&sun(), &camera, 1.0, field());
        let after = sun_cascades(&sun(), &moved, 1.0, field());
        let point = Vec3::new(3.0, 0.0, 10.0);

        for (a, b) in before.iter().zip(&after) {
            assert!((a.texel - b.texel).abs() < 1e-6);
            let half: f32 = MAP_SIZE.lossy_convert();
            let half = half / 2.0;
            let from = a.view_proj.project_point3(point);
            let to = b.view_proj.project_point3(point);
            for shift in [(from.x - to.x) * half, (from.y - to.y) * half] {
                assert!(
                    (shift - shift.round()).abs() < 1e-2,
                    "the map shifted {shift} texels"
                );
            }
        }
    }
}
