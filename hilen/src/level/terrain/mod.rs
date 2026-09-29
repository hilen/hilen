mod collision;
mod edges;
pub(crate) mod mesh;
mod shape;

#[cfg(test)]
mod carve_tests;
#[cfg(test)]
mod move_tests;

use geo::{Area, BooleanOps, BoundingRect, Coord, Intersects, Line, MultiPolygon, Polygon};
use rustc_hash::{FxHashMap, FxHashSet};

use self::{
    edges::{Edge, chunk_edges},
    shape::{ChunkKey, chunk_rect, chunks_in, circle, polygon, tidy, to_coord, union_all},
};
use crate::{
    deps::refs::{Own, Weak},
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Rect},
    },
    level::{LevelBase, LevelManager},
    window::image::{Image, ToImage},
};

/// A material of the ground, an index from `Terrain::add_material`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MaterialId(u16);

/// A strip drawn along every piece of ground outline flat enough to walk
/// on and open to the sky, like grass on dirt. The top of the image lies
/// on the outline and the strip hangs `height` level units down into the
/// ground. The image repeats along x every `height` times its aspect.
#[derive(Debug, Clone)]
pub struct TerrainSurface {
    pub image:  Weak<Image>,
    pub height: f32,
    /// Drawn over each end of a run of strip, like the rounded end of a
    /// grass ledge. Its left column lies on the end, so a right end draws
    /// it mirrored.
    pub end:    Option<TerrainSurfaceEnd>,
}

#[derive(Debug, Clone)]
pub struct TerrainSurfaceEnd {
    pub image: Weak<Image>,
    pub width: f32,
}

/// A flat color line along every piece of ground outline that has no
/// surface strip, like the dark outline of pixel art ground. It lies on
/// the outline and reaches `width` level units into the ground.
#[derive(Debug, Copy, Clone)]
pub struct TerrainEdge {
    pub color: Color,
    pub width: f32,
}

/// What a piece of ground looks like. The `fill` image repeats in level
/// space, `fill_size` level units wide and as tall as its aspect makes it.
#[derive(Debug, Clone)]
pub struct TerrainMaterial {
    pub fill:      Weak<Image>,
    pub fill_size: f32,
    pub surface:   Option<TerrainSurface>,
    pub edge:      Option<TerrainEdge>,
    /// How far in level units this fill spills over its borders with
    /// other materials, thinning out by a dither.
    pub blend:     Option<f32>,
}

impl TerrainMaterial {
    pub fn new(fill: impl ToImage, fill_size: f32) -> Self {
        Self {
            fill: fill.to_image(),
            fill_size,
            surface: None,
            edge: None,
            blend: None,
        }
    }

    pub fn with_blend(mut self, width: f32) -> Self {
        self.blend = Some(width);
        self
    }

    pub fn with_edge(mut self, color: impl Into<Color>, width: f32) -> Self {
        self.edge = Some(TerrainEdge {
            color: color.into(),
            width,
        });
        self
    }

    pub fn with_surface(mut self, image: impl ToImage, height: f32) -> Self {
        self.surface = Some(TerrainSurface {
            image: image.to_image(),
            height,
            end: None,
        });
        self
    }

    /// The end piece of the surface strip, `width` level units wide. Call
    /// after `with_surface`.
    pub fn with_surface_end(mut self, image: impl ToImage, width: f32) -> Self {
        let surface = self.surface.as_mut().expect("with_surface comes first");
        surface.end = Some(TerrainSurfaceEnd {
            image: image.to_image(),
            width,
        });
        self
    }
}

/// An invisible segment a box lands on only when it falls onto it from
/// above, like `TileCollision::Platform`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct OneWay {
    a: Point,
    b: Point,
}

#[derive(Debug)]
pub(super) struct Chunk {
    /// The ground of each material inside the chunk square, no two
    /// overlap.
    areas:   Vec<(MaterialId, MultiPolygon<f64>)>,
    /// Every material together, what a box collides with.
    solid:   MultiPolygon<f64>,
    edges:   Vec<Edge>,
    /// Goes up whenever the chunk has to be drawn again.
    version: u64,
}

/// Ground made of polygons with holes, in level units, that can be dug
/// into. Each piece of ground has a material. A box moves against the
/// ground outline with `move_box` the way it does on a `TileMap`, and
/// walks up and down any edge flatter than `slope_limit`.
#[derive(Debug)]
pub struct Terrain {
    materials: Vec<TerrainMaterial>,
    chunks:    FxHashMap<ChunkKey, Chunk>,
    one_way:   Vec<OneWay>,
    versions:  u64,

    /// The steepest edge in degrees a box still walks on. A steeper one is
    /// a wall. 50 by default.
    pub slope_limit: f32,
    /// Depth like `SpriteData::z_position`, `add_terrain` puts the ground
    /// in front of the tile maps and behind every sprite.
    pub z_position:  f32,
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

impl Terrain {
    pub fn new() -> Self {
        Self {
            materials:   vec![],
            chunks:      FxHashMap::default(),
            one_way:     vec![],
            versions:    0,
            slope_limit: 50.0,
            z_position:  LevelManager::default_z_position(),
        }
    }

    pub fn add_material(&mut self, material: TerrainMaterial) -> MaterialId {
        let id =
            MaterialId(u16::try_from(self.materials.len()).expect("a terrain takes at most 65536 materials"));
        self.materials.push(material);
        id
    }

    pub fn material(&self, id: MaterialId) -> &TerrainMaterial {
        &self.materials[usize::from(id.0)]
    }

    /// Adds ground of one material. `exterior` is the outline and each of
    /// `holes` cuts a hole in it, in any winding. Where the new ground
    /// overlaps older ground, the new material wins.
    pub fn add_polygon(&mut self, material: MaterialId, exterior: &[Point], holes: &[Vec<Point>]) {
        assert!(
            usize::from(material.0) < self.materials.len(),
            "material {} is not a material of this terrain",
            material.0
        );
        let shape = polygon(exterior, holes);
        let Some(bounds) = shape.bounding_rect() else {
            return;
        };
        let mut changed = vec![];
        for key in chunks_in(bounds.min(), bounds.max()) {
            let piece = shape.intersection(&chunk_rect(key).to_polygon());
            if piece.unsigned_area() <= 0.0 {
                continue;
            }
            let chunk = self.chunks.entry(key).or_insert_with(|| Chunk {
                areas:   vec![],
                solid:   MultiPolygon::new(vec![]),
                edges:   vec![],
                version: 0,
            });
            for (id, area) in &mut chunk.areas {
                *area = if *id == material {
                    area.union(&piece)
                } else {
                    area.difference(&piece)
                };
            }
            if !chunk.areas.iter().any(|(id, _)| *id == material) {
                chunk.areas.push((material, piece));
            }
            changed.push(key);
        }
        self.rebuild(&changed);
    }

    /// Removes the polygon from the ground of one material, other
    /// materials keep theirs. True when any ground was removed.
    pub fn carve_polygon(&mut self, points: &[Point], material: MaterialId) -> bool {
        let cut = polygon(points, &[]);
        let Some(bounds) = cut.bounding_rect() else {
            return false;
        };
        let mut changed = vec![];
        for key in chunks_in(bounds.min(), bounds.max()) {
            let Some(chunk) = self.chunks.get_mut(&key) else {
                continue;
            };
            for (id, area) in &mut chunk.areas {
                if *id != material || !touches(area, &cut) {
                    continue;
                }
                let before = area.unsigned_area();
                let after = area.difference(&cut);
                if before - after.unsigned_area() > 1e-9 {
                    *area = after;
                    changed.push(key);
                }
            }
        }
        self.rebuild(&changed);
        !changed.is_empty()
    }

    /// Removes a circle from the ground of one material, see
    /// `carve_polygon`.
    pub fn carve_circle(&mut self, center: impl Into<Point>, radius: f32, material: MaterialId) -> bool {
        self.carve_polygon(&circle(center.into(), radius), material)
    }

    /// The material of the ground at a level point, none in the air.
    pub fn material_at(&self, point: impl Into<Point>) -> Option<MaterialId> {
        let point = to_coord(point.into());
        let chunk = self.chunks.get(&chunks_in(point, point).next()?)?;
        chunk.areas.iter().find(|(_, area)| area.intersects(&point)).map(|(id, _)| *id)
    }

    /// Adds an invisible one way segment. A box lands on it only falling
    /// from above, passes it moving up or sideways, and falls through it
    /// with `drop_through`, like `TileCollision::Platform`. It holds the
    /// box at any slope.
    pub fn add_one_way(&mut self, a: impl Into<Point>, b: impl Into<Point>) {
        self.one_way.push(OneWay {
            a: a.into(),
            b: b.into(),
        });
    }

    /// Tidies the changed chunks, then works out the outline again for
    /// them and the chunks around them, whose outline along the shared
    /// border can change with them.
    fn rebuild(&mut self, changed: &[ChunkKey]) {
        let changed: FxHashSet<ChunkKey> = changed.iter().copied().collect();
        for key in &changed {
            let chunk = self.chunks.get_mut(key).expect("a changed chunk exists");
            for (_, area) in &mut chunk.areas {
                *area = tidy(area, *key);
            }
            chunk.areas.retain(|(_, area)| !area.0.is_empty());
            chunk.solid = union_all(chunk.areas.iter().map(|(_, area)| area));
        }
        self.chunks.retain(|_, chunk| !chunk.areas.is_empty());

        let mut outline: FxHashSet<ChunkKey> = FxHashSet::default();
        for &(x, y) in &changed {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    outline.insert((x + dx, y + dy));
                }
            }
        }
        for key in outline {
            if !self.chunks.contains_key(&key) {
                continue;
            }
            let edges = chunk_edges(key, &self.chunks[&key].solid, |other| {
                self.chunks.get(&other).map(|chunk| &chunk.solid)
            });
            let chunk = self.chunks.get_mut(&key).expect("checked above");
            // The outline band of a chunk is cut from its neighbors too, so
            // every chunk around a change is drawn again.
            self.versions += 1;
            chunk.edges = edges;
            chunk.version = self.versions;
        }
    }

    /// Whether no ground lies straight above `point`, so a surface like
    /// grass grows there. A tunnel floor is covered, a dug pit is not.
    pub(super) fn open_to_sky(&self, point: Point) -> bool {
        let from = to_coord(point);
        !self
            .chunks
            .iter()
            .filter(|(key, _)| {
                let rect = chunk_rect(**key);
                rect.min().x <= from.x && from.x <= rect.max().x && rect.max().y > from.y
            })
            .any(|(key, chunk)| {
                let top = Coord {
                    x: from.x,
                    y: chunk_rect(*key).max().y,
                };
                chunk.solid.intersects(&Line::new(from, top))
            })
    }

    /// The ground of a chunk and the 8 around it together. An outline band
    /// is cut from it, so a chunk border never reads as a side of the
    /// ground.
    pub(super) fn solid_around(&self, key: ChunkKey) -> MultiPolygon<f64> {
        union_all(
            (-1..=1)
                .flat_map(|dx| (-1..=1).map(move |dy| (key.0 + dx, key.1 + dy)))
                .filter_map(|near| self.chunks.get(&near).map(|chunk| &chunk.solid)),
        )
    }

    /// The ground of each material in a chunk and the 8 around it.
    pub(super) fn materials_around(&self, key: ChunkKey) -> Vec<(MaterialId, MultiPolygon<f64>)> {
        let mut around: Vec<(MaterialId, MultiPolygon<f64>)> = vec![];
        for near in (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (key.0 + dx, key.1 + dy))) {
            let Some(chunk) = self.chunks.get(&near) else {
                continue;
            };
            for (material, area) in &chunk.areas {
                match around.iter_mut().find(|(id, _)| id == material) {
                    Some((_, sum)) => *sum = sum.union(area),
                    None => around.push((*material, area.clone())),
                }
            }
        }
        around
    }

    /// The chunks whose square touches the rect, with their key.
    fn chunks_near(&self, min: Point, max: Point) -> impl Iterator<Item = (ChunkKey, &Chunk)> {
        chunks_in(to_coord(min), to_coord(max))
            .filter_map(|key| self.chunks.get(&key).map(|chunk| (key, chunk)))
    }

    fn edges_near(&self, min: Point, max: Point) -> impl Iterator<Item = &Edge> {
        self.chunks_near(min, max).flat_map(|(_, chunk)| &chunk.edges)
    }

    /// The key and draw version of every chunk that touches `rect`.
    pub(crate) fn visible_chunks(&self, rect: Rect) -> impl Iterator<Item = ((i32, i32), u64)> {
        self.chunks_near(rect.origin, Point::new(rect.max_x(), rect.max_y()))
            .map(|(key, chunk)| (key, chunk.version))
    }

    /// The version of a chunk, none once it has no ground left.
    pub(crate) fn chunk_version(&self, key: (i32, i32)) -> Option<u64> {
        self.chunks.get(&key).map(|chunk| chunk.version)
    }

    /// The tallest surface strip, how far below a chunk its drawing can
    /// reach.
    pub(crate) fn surface_reach(&self) -> f32 {
        self.materials
            .iter()
            .filter_map(|material| material.surface.as_ref().map(|surface| surface.height))
            .fold(0.0, f32::max)
    }
}

fn touches(area: &MultiPolygon<f64>, cut: &Polygon<f64>) -> bool {
    match (area.bounding_rect(), cut.bounding_rect()) {
        (Some(a), Some(b)) => a.intersects(&b),
        _ => false,
    }
}

impl LevelBase {
    /// Adds ground, drawn in front of every tile map and behind every
    /// sprite, and in front of the terrains added before it.
    pub fn add_terrain(&mut self, mut terrain: Terrain) -> Weak<Terrain> {
        let layers: f32 = self.terrains.len().lossy_convert();
        terrain.z_position =
            LevelManager::default_z_position() + LevelManager::z_position_offset() * (3.0 - layers * 0.5);
        let terrain = Own::new(terrain);
        let weak = terrain.weak();
        self.terrains.push(terrain);
        weak
    }

    pub fn remove_terrain(&mut self, terrain: Weak<Terrain>) {
        self.terrains.retain(|own| own.raw() != terrain.raw());
    }

    pub fn terrains(&self) -> &[Own<Terrain>] {
        &self.terrains
    }
}

#[cfg(test)]
impl Terrain {
    fn point_count(&self) -> usize {
        self.chunks
            .values()
            .flat_map(|chunk| &chunk.areas)
            .map(|(_, area)| shape::point_count(area))
            .sum()
    }

    fn versions(&self) -> FxHashMap<ChunkKey, u64> {
        self.chunks.iter().map(|(key, chunk)| (*key, chunk.version)).collect()
    }
}
