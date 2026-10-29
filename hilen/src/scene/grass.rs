//! Grass that grows in patches around the player and goes when the player
//! walks away: square patches grown nearest first, each patch 1 buffer of
//! blades drawn as 1 instanced draw, a whole patch culled at once, and the
//! far blades left out by count.

use std::f32::consts::TAU;

use bytemuck::cast_slice;
use rustc_hash::FxHashMap;
use wgpu::{Buffer, BufferUsages};

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::{
        LossyConvert,
        color::{Color, U8Color},
        flat::Point,
        volume::{Mat4, Quat, Vec3, Vec4, Vertex3D},
    },
    render::{
        DeviceHelper, GrassDraw, MeshKey, MeshPipeline,
        data::{GrassBlade, MeshInstance},
        frustum::{self, Frustum},
    },
    scene::{Light, Material, MeshData, Model, pick_lights},
    window::Window,
};

/// How fast each of the 4 sines of the sway runs against the gust, see
/// `v_grass` in mesh.wgsl. No 2 of them line up, so the sway never repeats.
const WAVE_SPEEDS: [f32; 4] = [1.0, 0.3412, -1.0412, 1.334_57];
/// `sway_speed` counts 20 seconds as 1 unit of time.
const SWAY_TIME: f32 = 1.0 / 20.0;

/// The share of the thinning length a blade shrinks over before it is
/// gone, see `v_grass`.
const SHRINK_SHARE: f32 = 0.15;

/// What grows at one spot of the ground, see `Grass::new`.
#[derive(Clone, Copy, Debug)]
pub struct GrassSpot {
    /// The height of the ground.
    pub height: f32,
    /// The ground's normal, what a blade stands along, `Vec3::Y` on flat
    /// ground, so grass stands out of a slope.
    pub normal: Vec3,
    /// Multiplies the blade's own colors, encoded sRGB, white for none.
    pub tint:   Color,
}

/// The wind over the grass of a scene, along `direction` on the ground.
/// `strength` is 0 still and 1 a storm: it scales the lean, the gusts and
/// how fast they run.
#[derive(Clone, Copy, Debug)]
pub struct Wind {
    pub direction: Vec3,
    pub strength:  f32,
}

impl Default for Wind {
    fn default() -> Self {
        Self {
            direction: Vec3::X,
            strength:  0.5,
        }
    }
}

/// A field of grass that grows around the player, or around the camera
/// with no player. `ground` answers where a blade can grow: the height of
/// the ground at a point and its tint, or none for no grass there. It is
/// asked at most `spots_per_frame` times a frame, the nearest patches
/// first. Put it in `SceneBase::grass`.
///
/// Every blade is an instance of `model`, see `Grass::blade` for a plain
/// blade, drawn with `material` the way a node is, lit, shadowed and
/// fogged. Grass casts no shadow. Up to `thin_from` every blade is drawn,
/// from there to `distance` they go one by one, each shrinking away, so
/// the edge of the field has no line.
pub struct Grass {
    pub model:           Weak<Model>,
    pub material:        Material,
    /// Blades per square unit.
    pub density:         f32,
    /// The side of a patch.
    pub patch_size:      f32,
    pub distance:        f32,
    pub thin_from:       f32,
    /// The smallest and the largest scale of a blade, picked at random.
    pub sizes:           (f32, f32),
    /// How much darker a blade can be than its tint, picked at random, so
    /// a field is not 1 flat color.
    pub shade:           f32,
    /// How far the wind moves the top of a blade.
    pub sway_distance:   f32,
    /// How fast the gusts run.
    pub sway_speed:      f32,
    /// How far the player pushes a blade away and down.
    pub push_distance:   f32,
    /// The height in the mesh where a blade bends in full. Below it the
    /// bend falls off in a straight line to 0 at the root.
    pub bend_height:     f32,
    /// The phases of the 4 sines of the sway, kept under a full turn so
    /// the sway never jumps however long a scene runs.
    phases:              [f32; 4],
    pub seed:            u64,
    pub spots_per_frame: u32,
    /// Where the field's own axes sit in the world: a point of the field
    /// lands on `origin + turn * point`. The patches lie on the field's x
    /// and z, `ground` answers in them, and a blade stands along the
    /// field's y tilted to the ground's normal given in them. A game on a
    /// round planet sets them to the ground near the player, its y away
    /// from the center, and gives the curve as heights: over the 40 m a
    /// field reaches, a planet of 20 km drops 4 cm. `move_world` moves
    /// them along. The world's own axes by default.
    pub origin:          Vec3,
    pub turn:            Quat,

    ground:  Box<dyn Fn(f32, f32) -> Option<GrassSpot>>,
    patches: FxHashMap<(i32, i32), Patch>,
}

/// A grown patch. One with no blade keeps no buffer and is not grown
/// again while it stays in reach.
struct Patch {
    blades: Option<Buffer>,
    count:  u32,
    slot:   Slot,
    /// The middle of the patch, in the field's axes.
    origin: Vec3,
    /// The sphere around every blade, what the camera culls it by, its
    /// center in the field's axes.
    center: Vec3,
    radius: f32,
}

impl Grass {
    /// The defaults are a meadow grass: 10 unit patches, thinning from 15
    /// and gone at 40.
    pub fn new(model: Weak<Model>, ground: impl Fn(f32, f32) -> Option<GrassSpot> + 'static) -> Self {
        Self {
            model,
            material: Material {
                color: Color::hex("#4f8a2e"),
                roughness: 0.9,
                ..Material::default()
            },
            density: 12.0,
            patch_size: 10.0,
            distance: 40.0,
            thin_from: 15.0,
            sizes: (0.7, 1.3),
            shade: 0.3,
            sway_distance: 2.3,
            sway_speed: 60.0,
            push_distance: 2.0,
            bend_height: 0.5,
            phases: [0.0; 4],
            seed: 0,
            spots_per_frame: 2048,
            origin: Vec3::ZERO,
            turn: Quat::IDENTITY,
            ground: Box::new(ground),
            patches: FxHashMap::default(),
        }
    }

    /// A blade `height` tall and `width` wide at its root, 3 segments
    /// that narrow and bend forward, darker at the root, seen from both
    /// sides, stored under `name`. Call it on the main thread.
    pub fn blade(name: &str, height: f32, width: f32) -> Weak<Model> {
        Model::from_mesh(name, blade_mesh(height, width))
    }

    /// A tuft of `blades` blades like `Grass::blade` from 1 root, fanning
    /// out, each leaning away and of its own height, the tallest `height`.
    /// A field of tufts reads as grass from far, single blades thin to
    /// dashes there.
    pub fn tuft(name: &str, height: f32, width: f32, blades: u32) -> Weak<Model> {
        Model::from_mesh(name, tuft_mesh(height, width, blades))
    }

    /// The patches grown so far, empty ones too.
    pub fn patch_count(&self) -> usize {
        self.patches.len()
    }

    /// The blades of every grown patch.
    pub fn blade_count(&self) -> u32 {
        self.patches.values().map(|patch| patch.count).sum()
    }

    /// Lets every patch grow again, after the ground or `ground` changed.
    pub fn regrow(&mut self) {
        self.patches.clear();
    }

    /// Lets the patches that touch the circle at `center` grow again, like
    /// after a floor was built there.
    pub fn regrow_around(&mut self, center: Vec3, radius: f32) {
        let center = self.to_field(center);
        let size = self.patch_size;
        self.patches.retain(|&cell, _| nearest_distance(center, cell, size) > radius);
    }

    /// Runs the sway on by `frame_time` seconds in `wind`, faster in a
    /// stronger wind.
    pub(crate) fn advance(&mut self, frame_time: f32, wind: Wind) {
        let run = self.sway_speed * frame_time * SWAY_TIME * (0.5 + 0.5 * wind.strength);
        for (phase, speed) in self.phases.iter_mut().zip(WAVE_SPEEDS) {
            *phase = (*phase + run * speed).rem_euclid(TAU);
        }
    }

    /// Drops the patches out of reach of `center` and grows the missing
    /// ones nearest first, until the frame's spots run out.
    /// A world point in the field's own axes.
    pub fn to_field(&self, world: Vec3) -> Vec3 {
        self.turn.inverse() * (world - self.origin)
    }

    /// A point of the field in the world.
    pub fn to_world(&self, field: Vec3) -> Vec3 {
        self.origin + self.turn * field
    }

    pub(crate) fn update(&mut self, center: Vec3) {
        let center = self.to_field(center);
        let size = self.patch_size;
        let keep = self.distance + size;
        self.patches.retain(|&cell, _| nearest_distance(center, cell, size) <= keep);

        let middle: (i32, i32) = (
            (center.x / size).floor().lossy_convert(),
            (center.z / size).floor().lossy_convert(),
        );
        let reach: i32 = (self.distance / size).ceil().lossy_convert();
        let mut spots = self.spots_per_frame;
        for (dx, dz) in rings(reach + 1) {
            if spots == 0 {
                return;
            }
            let cell = (middle.0 + dx, middle.1 + dz);
            if self.patches.contains_key(&cell) || nearest_distance(center, cell, size) > self.distance {
                continue;
            }
            let patch = self.grow(cell);
            spots = spots.saturating_sub(self.spots_in_patch());
            self.patches.insert(cell, patch);
        }
    }

    fn spots_in_patch(&self) -> u32 {
        (self.density * self.patch_size * self.patch_size).round().lossy_convert()
    }

    fn grow(&self, cell: (i32, i32)) -> Patch {
        let slot = Slot::take();
        let origin = cell_origin(cell, self.patch_size);
        let blades = plant(self, cell, slot.0);
        let tall = if self.model.is_ok() {
            self.model.bounds.max.y.max(0.0) * self.sizes.1
        } else {
            0.0
        };
        let low = blades.iter().map(|blade| blade.place[1]).fold(f32::INFINITY, f32::min);
        let high = blades.iter().map(|blade| blade.place[1]).fold(f32::NEG_INFINITY, f32::max);
        let half = self.patch_size / 2.0;
        let (center, radius) = if blades.is_empty() {
            (origin, 0.0)
        } else {
            let rise = f32::midpoint(high - low, tall);
            // A blade can hang out of the square by its own height.
            let side = half + tall;
            (
                origin + Vec3::Y * (low + rise),
                (2.0 * side * side + rise * rise).sqrt(),
            )
        };
        Patch {
            blades: (!blades.is_empty())
                .then(|| Window::device().buffer_from_bytes(cast_slice(&blades), BufferUsages::VERTEX)),
            count: blades.len().try_into().expect("blade count fits u32"),
            slot,
            origin,
            center,
            radius,
        }
    }

    /// Puts the patches the camera at `camera` sees through `view` into
    /// the pipeline, `view` taking points from the camera.
    pub(crate) fn push_draws(
        &self,
        pipeline: &mut MeshPipeline,
        camera: Vec3,
        view: &Frustum,
        lights: &[Light],
    ) {
        if !self.model.is_ok() {
            return;
        }
        let length = (self.distance - self.thin_from).max(1e-3);
        let shrink = (length * SHRINK_SHARE).max(1e-3);
        let cull = frustum::enabled();
        for patch in self.patches.values() {
            let Some(blades) = &patch.blades else {
                continue;
            };
            let world_center = self.to_world(patch.center);
            let world_origin = self.to_world(patch.origin);
            let offset = world_center - camera;
            if cull && !view.holds(offset, patch.radius) {
                continue;
            }
            let near = (offset.length() - patch.radius).max(0.0);
            let count = drawn_count(patch.count, near, self.thin_from, length);
            if count == 0 {
                continue;
            }
            let model = Mat4::from_rotation_translation(self.turn, world_origin);
            let instance = MeshInstance::new(
                model,
                self.material,
                pick_lights(world_center, patch.radius, lights),
                0,
            );
            let fade = Vec4::new(self.thin_from, length, 1.0 / shrink, patch.count.lossy_convert());
            // The 4th numbers of the sway and the place carry the patch's
            // place in the field, which the gusts run over.
            let sway = Vec4::new(
                self.sway_distance,
                self.push_distance,
                1.0 / self.bend_height.max(1e-4),
                patch.origin.z,
            );
            for part in &self.model.parts {
                pipeline.add_grass(GrassDraw {
                    key: MeshKey {
                        mesh:                   part.mesh.weak(),
                        texture:                self.material.texture,
                        normal_map:             self.material.normal_map,
                        metallic_roughness_map: self.material.metallic_roughness_map,
                        occlusion_map:          self.material.occlusion_map,
                        cutout:                 self.material.cutout.is_some(),
                    },
                    blades: blades.clone(),
                    count,
                    slot: patch.slot.0,
                    instance,
                    patch: Mat4::from_cols(
                        fade,
                        Vec4::from(self.phases),
                        sway,
                        world_origin.extend(patch.origin.x),
                    ),
                    turn: Mat4::from_quat(self.turn),
                });
            }
        }
    }
}

/// The middle of the patch `cell` at height 0.
fn cell_origin(cell: (i32, i32), size: f32) -> Vec3 {
    let x: f32 = cell.0.lossy_convert();
    let z: f32 = cell.1.lossy_convert();
    Vec3::new((x + 0.5) * size, 0.0, (z + 0.5) * size)
}

/// How far `center` is from the nearest point of the patch `cell`, along
/// the ground.
fn nearest_distance(center: Vec3, cell: (i32, i32), size: f32) -> f32 {
    let origin = cell_origin(cell, size);
    let half = size / 2.0;
    let dx = ((center.x - origin.x).abs() - half).max(0.0);
    let dz = ((center.z - origin.z).abs() - half).max(0.0);
    (dx * dx + dz * dz).sqrt()
}

/// The cells around the middle one, ring by ring outward, the middle
/// first, each cell once.
fn rings(reach: i32) -> impl Iterator<Item = (i32, i32)> {
    (0..=reach).flat_map(|ring| {
        (-ring..=ring).flat_map(move |dx| {
            (-ring..=ring).filter_map(move |dz| (dx.abs() == ring || dz.abs() == ring).then_some((dx, dz)))
        })
    })
}

/// The first blades of a patch of `count` that are drawn when its nearest
/// point is `near` from the camera. Blade `rank` is gone past
/// `thin_from + (1 - rank / count) * length`, see `v_grass`.
fn drawn_count(count: u32, near: f32, thin_from: f32, length: f32) -> u32 {
    let gone = ((near - thin_from) / length).clamp(0.0, 1.0);
    let total: f32 = count.lossy_convert();
    let drawn: u32 = (total * (1.0 - gone)).ceil().lossy_convert();
    drawn.min(count)
}

/// The blades of the patch `cell`, the same for the same seed and cell.
/// Every random draw is taken for every spot, so a spot with no grass
/// does not move the blades after it.
fn plant(grass: &Grass, cell: (i32, i32), slot: u32) -> Vec<GrassBlade> {
    let size = grass.patch_size;
    let origin = cell_origin(cell, size);
    let mut rng = fastrand::Rng::with_seed(patch_seed(grass.seed, cell));
    let mut blades = vec![];
    for _ in 0..grass.spots_in_patch() {
        let x = (rng.f32() - 0.5) * size;
        let z = (rng.f32() - 0.5) * size;
        let turn = rng.f32() * TAU;
        let scale = grass.sizes.0 + rng.f32() * (grass.sizes.1 - grass.sizes.0);
        let light = 1.0 - rng.f32() * grass.shade;
        if let Some(spot) = (grass.ground)(origin.x + x, origin.z + z) {
            let tint = spot.tint;
            blades.push(GrassBlade {
                place: [x, spot.height, z, turn],
                size: scale,
                tint: Color::rgba(tint.r * light, tint.g * light, tint.b * light, tint.a).into(),
                slot,
                normal: GrassBlade::pack_normal(spot.normal),
            });
        }
    }
    blades
}

fn patch_seed(seed: u64, (x, z): (i32, i32)) -> u64 {
    let x = u64::from(x.cast_unsigned());
    let z = u64::from(z.cast_unsigned());
    seed ^ (x << 32 | z).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// The colors of the 3 segments of a blade, root to tip. The root only a
/// little darker: a dark root on ground of the same green reads as a hole.
const SEGMENT_COLORS: [&str; 3] = ["#b4b4b4", "#dadada", "#ffffff"];

/// The radians between 2 blades of a tuft, the golden angle, so no 2
/// blades of a tuft line up.
const GOLDEN_ANGLE: f32 = 2.399_963;

/// The mesh of `Grass::blade`, 1 upright blade.
fn blade_mesh(height: f32, width: f32) -> MeshData {
    tuft_mesh(height, width, 1)
}

/// The mesh of `Grass::tuft`: `blades` blades from 1 root, each turned by
/// the golden angle, leaning out and of its own height. Each segment has
/// its own corners, a triangle takes 1 color, and the back is its own
/// triangles, back faces are culled.
fn tuft_mesh(height: f32, width: f32, blades: u32) -> MeshData {
    let mut mesh = MeshData::default();
    for blade in 0..blades {
        let index: f32 = blade.lossy_convert();
        let (turn, lean, tall) = if blades == 1 {
            (0.0, 0.0, 1.0)
        } else {
            (
                index * GOLDEN_ANGLE,
                0.25 + 0.4 * (index * 0.618_034).fract(),
                0.65 + 0.35 * (index * 0.381_966 + 0.5).fract(),
            )
        };
        let turned = Quat::from_rotation_y(turn);
        let rotation = turned * Quat::from_rotation_x(lean);
        let root = turned * Vec3::Z * (width * 0.3 * lean);
        push_blade(&mut mesh, height * tall, width, |point| root + rotation * point);
    }
    mesh
}

/// 1 blade of `height` and `width`, its points placed by `place`.
fn push_blade(mesh: &mut MeshData, height: f32, width: f32, place: impl Fn(Vec3) -> Vec3) {
    let level = |step: f32| {
        let t = step / 3.0;
        let half = width / 2.0 * (1.0 - 0.6 * t);
        let bend = 0.15 * height * t * t;
        (half, t * height, bend)
    };
    for (segment, color) in SEGMENT_COLORS.iter().enumerate() {
        let color: U8Color = Color::hex(color).into();
        let step: f32 = segment.lossy_convert();
        let (low_half, low_y, low_z) = level(step);
        let (high_half, high_y, high_z) = level(step + 1.0);
        let mut corners = vec![
            Vec3::new(-low_half, low_y, low_z),
            Vec3::new(low_half, low_y, low_z),
        ];
        if segment == 2 {
            corners.push(Vec3::new(0.0, high_y, high_z));
        } else {
            corners.push(Vec3::new(high_half, high_y, high_z));
            corners.push(Vec3::new(-high_half, high_y, high_z));
        }
        let corners: Vec<Vec3> = corners.into_iter().map(&place).collect();
        let front = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalize();
        for side in [1.0, -1.0] {
            let base: u32 = mesh.vertices.len().try_into().expect("tuft fits u32");
            for (corner, v) in corners.iter().zip([0.0, 0.0, 1.0, 1.0]) {
                mesh.vertices.push(Vertex3D {
                    pos: *corner,
                    normal: front * side,
                    uv: Point::new(0.5, v),
                    color,
                });
            }
            let fan: &[[u32; 3]] = if corners.len() == 3 {
                &[[0, 1, 2]]
            } else {
                &[[0, 1, 2], [0, 2, 3]]
            };
            for [a, b, c] in fan {
                if side > 0.0 {
                    mesh.indices.extend([base + a, base + b, base + c]);
                } else {
                    mesh.indices.extend([base + a, base + c, base + b]);
                }
            }
        }
    }
}

/// A free slot of the frame's grass buffers, held by a patch while it
/// lives and given back when it goes.
struct Slot(u32);

#[derive(Default)]
struct Slots {
    free: Vec<u32>,
    next: u32,
}

static SLOTS: MainLock<Slots> = MainLock::new();

impl Slot {
    fn take() -> Self {
        let slots = SLOTS.get_mut();
        Self(slots.free.pop().unwrap_or_else(|| {
            slots.next += 1;
            slots.next - 1
        }))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        SLOTS.get_mut().free.push(self.0);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gm::color::WHITE;

    fn field(ground: impl Fn(f32, f32) -> Option<GrassSpot> + 'static) -> Grass {
        let mut grass = Grass::new(Weak::default(), ground);
        grass.patch_size = 8.0;
        grass
    }

    /// Grass everywhere on flat ground 2 up.
    fn flat() -> impl Fn(f32, f32) -> Option<GrassSpot> {
        |_, _| {
            Some(GrassSpot {
                height: 2.0,
                normal: Vec3::Y,
                tint:   WHITE,
            })
        }
    }

    /// A field turned onto the side of a planet: its y points along the
    /// world's x, so a point 2 up in the field lies 2 along x in the world.
    #[test]
    fn a_field_has_its_own_axes() {
        let mut grass = field(flat());
        grass.origin = Vec3::new(100.0, 0.0, 5.0);
        grass.turn = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
        let world = grass.to_world(Vec3::new(0.0, 2.0, 3.0));
        assert!((world - Vec3::new(102.0, 0.0, 8.0)).length() < 1e-4, "{world}");
        assert!((grass.to_field(world) - Vec3::new(0.0, 2.0, 3.0)).length() < 1e-4);
    }

    #[test]
    fn a_seed_plants_the_same_blades() {
        let grass = field(flat());
        assert_eq!(plant(&grass, (3, -2), 0), plant(&grass, (3, -2), 0));
        assert_ne!(plant(&grass, (3, -2), 0), plant(&grass, (3, -1), 0));
        let mut other = field(flat());
        other.seed = 7;
        assert_ne!(plant(&grass, (3, -2), 0), plant(&other, (3, -2), 0));
    }

    #[test]
    fn blades_stay_in_their_patch_on_the_ground() {
        let grass = field(flat());
        let blades = plant(&grass, (-1, 4), 5);
        assert_eq!(blades.len(), 768);
        for blade in blades {
            assert!(blade.place[0].abs() <= 4.0 && blade.place[2].abs() <= 4.0);
            assert_eq!(blade.place[1], 2.0);
            assert_eq!(blade.slot, 5);
            assert!((0.7..=1.3).contains(&blade.size));
        }
    }

    // A spot with no grass takes its random draws too, so cutting a hole
    // in a field leaves the blades around it where they were.
    #[test]
    fn a_hole_keeps_the_other_blades() {
        // The patch west of the origin, x from -8 to 0, holed in its west half.
        let full = plant(&field(flat()), (-1, 0), 0);
        let holed = plant(
            &field(|x, _| {
                (x > -4.0).then_some(GrassSpot {
                    height: 2.0,
                    normal: Vec3::Y,
                    tint:   WHITE,
                })
            }),
            (-1, 0),
            0,
        );
        let kept: Vec<_> = full.iter().filter(|blade| blade.place[0] > 0.0).copied().collect();
        assert!(!holed.is_empty() && holed.len() < full.len());
        assert_eq!(holed, kept);
    }

    #[test]
    fn rings_go_outward_and_cover_each_cell_once() {
        let cells: Vec<_> = rings(3).collect();
        assert_eq!(cells.len(), 49);
        assert_eq!(cells[0], (0, 0));
        let ring = |(x, z): (i32, i32)| x.abs().max(z.abs());
        assert!(cells.windows(2).all(|pair| ring(pair[0]) <= ring(pair[1])));
        let mut sorted = cells.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 49);
    }

    #[test]
    fn far_patches_draw_fewer_blades() {
        assert_eq!(drawn_count(100, 0.0, 20.0, 20.0), 100);
        assert_eq!(drawn_count(100, 20.0, 20.0, 20.0), 100);
        assert_eq!(drawn_count(100, 30.0, 20.0, 20.0), 50);
        assert_eq!(drawn_count(100, 39.9, 20.0, 20.0), 1);
        assert_eq!(drawn_count(100, 40.0, 20.0, 20.0), 0);
    }

    #[test]
    fn the_nearest_point_of_a_patch() {
        assert_eq!(nearest_distance(Vec3::new(4.0, 9.0, 4.0), (0, 0), 8.0), 0.0);
        assert_eq!(nearest_distance(Vec3::new(11.0, 0.0, 4.0), (0, 0), 8.0), 3.0);
        assert_eq!(nearest_distance(Vec3::new(-3.0, 0.0, -4.0), (0, 0), 8.0), 5.0);
    }

    // The faces of the front look along +z and the back along -z, each
    // wound counter clockwise from where it is seen, every triangle in 1
    // color, which `Model::from_mesh` needs.
    #[test]
    fn the_blade_has_a_front_and_a_back() {
        let mesh = blade_mesh(0.5, 0.06);
        assert_eq!(mesh.indices.len(), 30);
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|corner| mesh.vertices[triangle[corner] as usize]);
            assert!(a.color == b.color && b.color == c.color);
            let facing = (b.pos - a.pos).cross(c.pos - a.pos).normalize();
            assert!(facing.dot(a.normal) > 0.9, "{facing} against {}", a.normal);
        }
        let top = mesh.vertices.iter().map(|vertex| vertex.pos.y).fold(0.0, f32::max);
        assert!((top - 0.5).abs() < 1e-6);
    }

    // Every blade of a tuft faces out from both sides, leans away from
    // the root and stays under the height asked for.
    #[test]
    fn a_tuft_fans_out() {
        let mesh = tuft_mesh(0.6, 0.08, 5);
        assert_eq!(mesh.indices.len(), 5 * 30);
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|corner| mesh.vertices[triangle[corner] as usize]);
            assert!(a.color == b.color && b.color == c.color);
            let facing = (b.pos - a.pos).cross(c.pos - a.pos).normalize();
            assert!(facing.dot(a.normal) > 0.99, "{facing} against {}", a.normal);
        }
        let top = mesh.vertices.iter().map(|vertex| vertex.pos.y).fold(0.0, f32::max);
        assert!(top <= 0.6 && top > 0.4, "{top}");
        let reach = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x.hypot(vertex.pos.z))
            .fold(0.0, f32::max);
        assert!(reach > 0.1, "{reach}");
    }
}
