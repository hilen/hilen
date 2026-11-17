use std::sync::Arc;

use hilen::{
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{ColliderShape, Heightfield, Model, Node, NodeTemplates, Prop, SceneCreation, Wall},
    ui::Color,
};

use super::{
    TerrainShadows,
    meshes::{pick, tree},
};

/// Chunks of ground loaded around the player, each its own mesh with a
/// height every meter, 5 by 5 like a streaming world.
pub const CHUNK: f32 = 64.0;
const CHUNK_POINTS: usize = 65;
const LOAD_RADIUS: i32 = 2;
/// The coarse ground past the chunks, a height every 10 m, a little under,
/// built again after this much flying.
const FAR_SIZE: f32 = 2400.0;
const FAR_POINTS: usize = 241;
const FAR_DROP: f32 = 3.0;
const FAR_REBUILD: f32 = 256.0;
const SEA_LEVEL: f32 = -6.0;
/// About 10 trees and 4 items a chunk, the counts of geblings.
const TREES: usize = 10;
const ROCKS: usize = 4;
pub const TREE_SHAPES: usize = 32;

/// Steep bumpy ground about 12 m high on hills 20 m high, everywhere.
pub fn hills(x: f32, z: f32) -> f32 {
    6.0 * (x / 17.0).sin() * (z / 13.0).cos()
        + 3.5 * ((x + 0.6 * z) / 7.0).sin()
        + 2.0 * ((z - 0.8 * x) / 4.3).cos()
        + 1.0 * (x / 2.1).sin() * (z / 2.7).sin()
        + 8.0 * (x / 61.0).sin() * (z / 47.0).sin()
}

/// What stands in 1 loaded chunk, its ground first.
pub struct Chunk {
    name:  String,
    nodes: Vec<Weak<dyn Node>>,
}

fn chunk_of(at: Vec3) -> (i32, i32) {
    (
        (at.x / CHUNK).round().lossy_convert(),
        (at.z / CHUNK).round().lossy_convert(),
    )
}

impl TerrainShadows {
    /// Adds the chunks near `at` that are missing and takes out the ones
    /// too far, then moves the far ground and the sea along.
    pub fn stream(&mut self, at: Vec3) {
        let (cx, cz) = chunk_of(at);
        let gone: Vec<(i32, i32)> = self
            .chunks
            .keys()
            .filter(|(x, z)| (x - cx).abs().max((z - cz).abs()) > LOAD_RADIUS + 1)
            .copied()
            .collect();
        for key in gone {
            if let Some(chunk) = self.chunks.remove(&key) {
                for node in chunk.nodes {
                    self.remove(node);
                }
                Model::free_with_name(chunk.name);
            }
        }
        for x in cx - LOAD_RADIUS..=cx + LOAD_RADIUS {
            for z in cz - LOAD_RADIUS..=cz + LOAD_RADIUS {
                if !self.chunks.contains_key(&(x, z)) {
                    let chunk = self.add_chunk(x, z);
                    self.chunks.insert((x, z), chunk);
                }
            }
        }
        let far_from = self.far.as_ref().map_or(f32::INFINITY, |(_, center)| (*center - at).length());
        if far_from > FAR_REBUILD {
            self.build_far(Vec3::new(
                (at.x / 10.0).round() * 10.0,
                0.0,
                (at.z / 10.0).round() * 10.0,
            ));
        }
    }

    fn add_chunk(&mut self, cx: i32, cz: i32) -> Chunk {
        let place = |c: i32| -> f32 { c.lossy_convert() };
        let center = Vec3::new(place(cx) * CHUNK, 0.0, place(cz) * CHUNK);
        let field = Heightfield::from_fn(CHUNK_POINTS, CHUNK_POINTS, CHUNK, CHUNK, |x, z| {
            hills(x + center.x, z + center.z)
        });
        let name = format!("Terrain shadows chunk {cx} {cz}");
        let mesh = field.mesh();
        let mut ground = self.make_node::<Wall>(Shape3::Model(Model::from_mesh(&name, mesh)), center);
        ground
            .set_collider(ColliderShape::Heightfield(Arc::new(field)), Vec3::ZERO)
            .set_color(Color::hex("#4f9a3a"))
            .set_roughness(1.0);
        let mut nodes = vec![ground.weak_node()];

        let seed = usize::try_from((cx.wrapping_mul(7919) ^ cz.wrapping_mul(104_729)).cast_unsigned())
            .expect("a u32 fits usize");
        let spot = |i: usize, salt: u32| {
            let x = center.x + (pick(seed + i, salt) - 0.5) * CHUNK;
            let z = center.z + (pick(seed + i, salt + 1) - 0.5) * CHUNK;
            Vec3::new(x, hills(x, z), z)
        };
        for i in 0..TREES {
            let at = spot(i, 11);
            let mut node = self.make_node::<Wall>(
                Shape3::Model(self.shapes[(seed + i) % TREE_SHAPES]),
                at - Vec3::Y * 0.6,
            );
            node.set_scale(2.0)
                .set_collider(
                    ColliderShape::Box(Vec3::new(0.6, 6.0, 0.6)),
                    Vec3::new(0.0, 3.0, 0.0),
                )
                .set_color(Color::rgb(1.0, 1.0, 1.0))
                .set_roughness(1.0);
            nodes.push(node.weak_node());
        }
        for i in 0..ROCKS {
            let mut node = self.make_node::<Wall>(Shape3::Ball(0.5 + pick(seed + i, 23) * 0.6), spot(i, 21));
            node.set_color(Color::hex("#8a8a92")).set_roughness(0.9);
            nodes.push(node.weak_node());
        }
        Chunk { name, nodes }
    }

    /// The coarse ground and the sea of the square around `center`, in
    /// place of the old ones.
    fn build_far(&mut self, center: Vec3) {
        self.far_builds += 1;
        let name = format!("Terrain shadows far ground {}", self.far_builds);
        let field = Heightfield::from_fn(FAR_POINTS, FAR_POINTS, FAR_SIZE, FAR_SIZE, |x, z| {
            hills(x + center.x, z + center.z) - FAR_DROP
        });
        let mut ground = self.make_node::<Prop>(Shape3::Model(Model::from_mesh(&name, field.mesh())), center);
        ground.set_color(Color::hex("#4a9035")).set_roughness(1.0);
        if let Some((old, _)) = self.far.replace((ground.weak_node(), center)) {
            self.remove(old);
            Model::free_with_name(format!("Terrain shadows far ground {}", self.far_builds - 1));
        }
        if let Some(mut sea) = self.sea {
            sea.set_position(center + Vec3::Y * SEA_LEVEL);
        } else {
            let mut sea = self.make_node::<Prop>(Shape3::Plane(FAR_SIZE), center + Vec3::Y * SEA_LEVEL);
            sea.set_color(Color::rgba(0.1, 0.3, 0.7, 0.75)).set_roughness(0.05);
            self.sea = Some(sea.weak_node());
        }
    }
}

/// The tree shapes every chunk draws its trees from.
pub fn tree_shapes() -> Vec<Weak<Model>> {
    (0..TREE_SHAPES)
        .map(|shape| Model::from_mesh(&format!("Terrain shadows tree {shape}"), tree(shape)))
        .collect()
}
