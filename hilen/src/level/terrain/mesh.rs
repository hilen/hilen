use geo::{BooleanOps, Buffer, MultiPolygon};
use lyon::{
    math::point,
    path::Path,
    tessellation::{BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers},
};

use super::{
    MaterialId, Terrain,
    edges::Edge,
    shape::{ChunkKey, polygon, to_point},
};
use crate::gm::flat::{Point, Vertex2D};

/// How far a surface split point may be off an edge and still split it.
const ON_EDGE: f32 = 0.001;
/// How far past a border a probe looks for the material on each side.
const PROBE: f32 = 0.01;

/// What a mesh part draws.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PartKind {
    /// The ground of a material, its image repeats in level space.
    Fill,
    /// The surface strip, like grass, along walkable edges.
    Surface,
    /// The end piece over each end of a run of surface strip.
    SurfaceEnd,
    /// The flat color band along the whole ground outline, under the
    /// surface strips.
    Edge,
    /// The fill of a material spilling over its border into another
    /// material, thinned out by a dither.
    Blend,
}

/// One draw of a chunk. A fill vertex needs only its position. A surface
/// or edge vertex carries in `uv.y` how far in from the outline it is, 0
/// on the outline and 1 on the far side.
#[derive(Debug, Clone)]
pub(crate) struct MeshPart {
    pub(crate) material: MaterialId,
    pub(crate) kind:     PartKind,
    pub(crate) vertices: Vec<Vertex2D>,
    pub(crate) indices:  Vec<u32>,
}

pub(crate) type ChunkMesh = Vec<MeshPart>;

fn vertex(pos: Point, v: f32) -> Vertex2D {
    Vertex2D {
        pos,
        uv: Point::new(0.0, v),
    }
}

fn fill(area: &MultiPolygon<f64>) -> (Vec<Vertex2D>, Vec<u32>) {
    let mut builder = Path::builder();
    let rings = area
        .iter()
        .flat_map(|polygon| std::iter::once(polygon.exterior()).chain(polygon.interiors()));
    for ring in rings {
        let mut coords = ring.0.iter().map(|coord| to_point(*coord));
        let Some(first) = coords.next() else {
            continue;
        };
        builder.begin(point(first.x, first.y));
        for p in coords {
            builder.line_to(point(p.x, p.y));
        }
        builder.end(true);
    }
    let path = builder.build();

    let mut geometry: VertexBuffers<Vertex2D, u32> = VertexBuffers::new();
    let result = FillTessellator::new().tessellate_path(
        &path,
        &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
            vertex_at(vertex.position().x, vertex.position().y)
        }),
    );
    if let Err(error) = result {
        log::error!("Terrain fill tessellation failed: {error:?}");
        return (vec![], vec![]);
    }
    (geometry.vertices, geometry.indices)
}

fn vertex_at(x: f32, y: f32) -> Vertex2D {
    vertex(Point::new(x, y), 0.0)
}

impl Terrain {
    /// The triangles of one chunk: a fill per material, a surface strip
    /// per material that has one along the original walkable outline, an
    /// edge band per material that has one, and a blend band per material
    /// that spills over its borders. Empty once the chunk has no ground.
    pub(crate) fn chunk_mesh(&self, key: ChunkKey) -> ChunkMesh {
        let Some(chunk) = self.chunks.get(&key) else {
            return vec![];
        };

        let mut parts: ChunkMesh = chunk
            .areas
            .iter()
            .map(|(material, area)| {
                let (vertices, indices) = fill(area);
                MeshPart {
                    material: *material,
                    kind: PartKind::Fill,
                    vertices,
                    indices,
                }
            })
            .filter(|part| !part.indices.is_empty())
            .collect();

        // An outline edge can run over two materials, split it where one
        // ends so each piece takes the look of the ground under it.
        let corners: Vec<Point> = chunk
            .areas
            .iter()
            .flat_map(|(_, area)| area.iter())
            .flat_map(|polygon| std::iter::once(polygon.exterior()).chain(polygon.interiors()))
            .flat_map(|ring| ring.0.iter().map(|coord| to_point(*coord)))
            .collect();

        // Strips hang and bands spill across chunk borders, so they clip to
        // the ground of each material in this chunk and the 8 around it.
        let around = self.materials_around(key);
        let grassy: Vec<(Point, Point, MaterialId)> = chunk
            .edges
            .iter()
            .flat_map(|edge| {
                self.pieces(edge, &corners)
                    .into_iter()
                    .filter(|(a, b, material)| self.grows_surface(edge, (*a + *b) / 2.0, *material))
            })
            .collect();
        for &(a, b, material) in &grassy {
            let Some(surface) = &self.material(material).surface else {
                continue;
            };
            Self::push_surface(&mut parts, &around, material, surface.height, a, b);
        }
        for &(a, b, material) in &grassy {
            for (end, other) in [(a, b), (b, a)] {
                let joined = grassy
                    .iter()
                    .any(|&(c, d, _)| (c, d) != (a, b) && (near(c, end) || near(d, end)));
                if !joined && !self.surface_goes_on(key, end) {
                    self.push_surface_end(&mut parts, &around, material, end, other);
                }
            }
        }

        // One band along the outline of the ground around the chunk, so
        // arcs and corners join and a chunk border gets no band.
        let solid = self.solid_around(key);
        for (material, area) in &chunk.areas {
            let Some(line) = self.material(*material).edge else {
                continue;
            };
            let band = solid.difference(&solid.buffer(-f64::from(line.width)));
            let (vertices, indices) = fill(&area.intersection(&band));
            push_mesh(&mut parts, *material, PartKind::Edge, vertices, indices);
        }

        for (material, area) in &chunk.areas {
            let Some(width) = self.material(*material).blend else {
                continue;
            };
            for (a, b) in rings(area).flat_map(|(a, b)| split(a, b, &corners)) {
                self.push_blend(&mut parts, &around, *material, width, a, b);
            }
        }

        parts
    }

    /// Whether the piece of `edge` around `at`, over ground of `material`,
    /// grows a surface strip: the edge is walkable, the material has a
    /// surface, and nothing covers it from the sky.
    fn grows_surface(&self, edge: &Edge, at: Point, material: MaterialId) -> bool {
        self.walkable(edge)
            && self.material(material).surface.is_some()
            && self.open_to_sky(at + edge.normal * PROBE)
    }

    /// Whether a surface strip in another chunk than `key` starts at
    /// `end`, so a run crossing a chunk border gets no end piece there.
    fn surface_goes_on(&self, key: ChunkKey, end: Point) -> bool {
        let reach = Point::new(ON_EDGE * 10.0, ON_EDGE * 10.0);
        self.chunks_near(end - reach, end + reach)
            .filter(|(other, _)| *other != key)
            .flat_map(|(_, chunk)| &chunk.edges)
            .any(|edge| {
                let from = if near(edge.a, end) {
                    edge.b
                } else if near(edge.b, end) {
                    edge.a
                } else {
                    return false;
                };
                let along = from - end;
                let inside = end + along * (0.01 / along.length().max(0.01));
                self.material_at(inside - edge.normal * PROBE)
                    .is_some_and(|material| self.grows_surface(edge, inside, material))
            })
    }

    /// The end piece at `end` of the run of strip that goes on toward
    /// `other`, as wide as the material says along the edge and hanging
    /// like the strip, clipped to the ground. Its u is 0 on the end and 1
    /// inward, so a right end draws the image mirrored.
    fn push_surface_end(
        &self,
        parts: &mut ChunkMesh,
        around: &[(MaterialId, MultiPolygon<f64>)],
        material: MaterialId,
        end: Point,
        other: Point,
    ) {
        let Some(surface) = &self.material(material).surface else {
            return;
        };
        let Some(piece) = &surface.end else {
            return;
        };
        let Some((_, area)) = around.iter().find(|(id, _)| *id == material) else {
            return;
        };
        let along = other - end;
        let length = along.length();
        if length < ON_EDGE || piece.width <= 0.0 {
            return;
        }
        let inward = along * (piece.width / length);
        let down = Point::new(0.0, surface.height);
        let tip = end + inward;
        let shape = polygon(&[end, tip, tip - down, end - down], &[]);
        let (mut vertices, indices) = fill(&area.intersection(&shape));
        let rise = tip.y - end.y;
        for v in &mut vertices {
            // The piece hangs straight down, so u runs along x only. Along the
            // slope, the lower corners of a sloped piece read past its far end.
            let u = ((v.pos.x - end.x) / inward.x).clamp(0.0, 1.0);
            let top = end.y + rise * u;
            v.uv = Point::new(u, ((top - v.pos.y) / surface.height).clamp(0.0, 1.0));
        }
        push_mesh(parts, material, PartKind::SurfaceEnd, vertices, indices);
    }

    /// The surface strip under the walkable piece from `a` to `b`, hanging
    /// `height` straight down and clipped to the ground of `material`, so
    /// it never shows in the air or under the next piece of outline. Each
    /// vertex carries in `uv.y` how far down the strip it is.
    fn push_surface(
        parts: &mut ChunkMesh,
        around: &[(MaterialId, MultiPolygon<f64>)],
        material: MaterialId,
        height: f32,
        a: Point,
        b: Point,
    ) {
        let Some((_, area)) = around.iter().find(|(id, _)| *id == material) else {
            return;
        };
        let down = Point::new(0.0, height);
        let strip = polygon(&[a, b, b - down, a - down], &[]);
        let (mut vertices, indices) = fill(&area.intersection(&strip));
        let run = b.x - a.x;
        for v in &mut vertices {
            let t = if run.abs() < ON_EDGE {
                0.0
            } else {
                (v.pos.x - a.x) / run
            };
            let top = a.y + (b.y - a.y) * t.clamp(0.0, 1.0);
            v.uv.y = ((top - v.pos.y) / height).clamp(0.0, 1.0);
        }
        push_mesh(parts, material, PartKind::Surface, vertices, indices);
    }

    /// The band of `material` spilling `width` units over the border piece
    /// from `a` to `b`, if another material lies across it. The band is
    /// clipped to that material, so it never shows in the air, and each
    /// vertex carries in `uv.y` how far across the band it is.
    fn push_blend(
        &self,
        parts: &mut ChunkMesh,
        around: &[(MaterialId, MultiPolygon<f64>)],
        material: MaterialId,
        width: f32,
        a: Point,
        b: Point,
    ) {
        let along = b - a;
        let length = along.length();
        if length < ON_EDGE {
            return;
        }
        let side = Point::new(along.y / length, -along.x / length);
        let mid = (a + b) / 2.0;
        let out = if self.material_at(mid + side * PROBE) == Some(material) {
            side * -1.0
        } else {
            side
        };
        let Some(other) = self.material_at(mid + out * PROBE) else {
            return;
        };
        if other == material {
            return;
        }
        let Some((_, other_area)) = around.iter().find(|(id, _)| *id == other) else {
            return;
        };
        let band = polygon(&[a, b, b + out * width, a + out * width], &[]);
        let (mut vertices, indices) = fill(&other_area.intersection(&band));
        if indices.is_empty() {
            return;
        }
        for v in &mut vertices {
            let offset = v.pos - a;
            v.uv.y = ((offset.x * out.x + offset.y * out.y) / width).clamp(0.0, 1.0);
        }
        push_mesh(parts, material, PartKind::Blend, vertices, indices);
    }
}

/// Whether 2 outline points are the same point.
fn near(a: Point, b: Point) -> bool {
    (a - b).length() < ON_EDGE * 10.0
}

/// Every side of every ring of `area`.
fn rings(area: &MultiPolygon<f64>) -> impl Iterator<Item = (Point, Point)> + '_ {
    area.iter()
        .flat_map(|polygon| std::iter::once(polygon.exterior()).chain(polygon.interiors()))
        .flat_map(|ring| ring.lines().map(|line| (to_point(line.start), to_point(line.end))))
}

/// The pieces of the segment from `a` to `b` between the ground corners
/// that lie on it.
fn split(a: Point, b: Point, corners: &[Point]) -> Vec<(Point, Point)> {
    let along = b - a;
    let length_sq = along.x * along.x + along.y * along.y;
    if length_sq <= 0.0 {
        return vec![];
    }
    let mut cuts: Vec<f32> = corners
        .iter()
        .filter_map(|corner| {
            let offset = *corner - a;
            let t = (offset.x * along.x + offset.y * along.y) / length_sq;
            let off = (offset - along * t).length();
            (t > 0.001 && t < 0.999 && off < ON_EDGE).then_some(t)
        })
        .collect();
    cuts.push(0.0);
    cuts.push(1.0);
    cuts.sort_by(f32::total_cmp);
    cuts.dedup_by(|x, y| (*x - *y).abs() < 0.001);
    cuts.windows(2).map(|pair| (a + along * pair[0], a + along * pair[1])).collect()
}

impl Terrain {
    /// The pieces of `edge` between the ground corners on it, each with the
    /// material of the ground just inside it. The probe asks the whole
    /// terrain, an edge a hair under a chunk border probes into the chunk
    /// above.
    fn pieces(&self, edge: &Edge, corners: &[Point]) -> Vec<(Point, Point, MaterialId)> {
        split(edge.a, edge.b, corners)
            .into_iter()
            .filter_map(|(a, b)| {
                let material = self.material_at((a + b) / 2.0 - edge.normal * PROBE)?;
                Some((a, b, material))
            })
            .collect()
    }
}

/// The part of that kind and material, a new empty one if there is none.
fn part_of(parts: &mut ChunkMesh, material: MaterialId, kind: PartKind) -> &mut MeshPart {
    let index = parts
        .iter()
        .position(|part| part.kind == kind && part.material == material)
        .unwrap_or_else(|| {
            parts.push(MeshPart {
                material,
                kind,
                vertices: vec![],
                indices: vec![],
            });
            parts.len() - 1
        });
    &mut parts[index]
}

/// Adds triangles to the part of that kind and material.
fn push_mesh(
    parts: &mut ChunkMesh,
    material: MaterialId,
    kind: PartKind,
    vertices: Vec<Vertex2D>,
    indices: Vec<u32>,
) {
    if indices.is_empty() {
        return;
    }
    let part = part_of(parts, material, kind);
    let first = u32::try_from(part.vertices.len()).expect("a chunk part has under 4 billion vertices");
    part.vertices.extend(vertices);
    part.indices.extend(indices.into_iter().map(|i| first + i));
}
