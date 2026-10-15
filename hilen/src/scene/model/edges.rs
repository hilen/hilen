use indexmap::IndexMap;

use super::MeshData;
use crate::gm::volume::Vec3;

/// Vertices closer than this are one place, a mesh with flat faces
/// repeats every corner once per face.
const WELD: f32 = 1e-4;

/// A place of a vertex on the weld grid, `-0` and `0` alike.
type Key = [u32; 3];

fn key(point: Vec3) -> Key {
    ((point / WELD).round() + Vec3::ZERO).to_array().map(f32::to_bits)
}

/// The two ends of an edge and the normal of every triangle on it.
struct Edge {
    ends:    [Vec3; 2],
    normals: Vec<Vec3>,
}

impl MeshData {
    /// The edges of the mesh worth a line: where two triangles meet at an
    /// angle of more than `crease_degrees`, and where a triangle has no
    /// neighbor, the rim of an open mesh. Flat parts and smooth curves
    /// give none, so a part drawn with these looks like a technical
    /// drawing. Vertices at one place count as one. Each edge once, in
    /// the order the triangles first name it. Put them on a node with
    /// `Line3`, see `NodeData::lines`.
    pub fn edges(&self, crease_degrees: f32) -> Vec<[Vec3; 2]> {
        let crease = crease_degrees.to_radians().cos();
        let mut edges: IndexMap<(Key, Key), Edge> = IndexMap::new();

        for triangle in self.indices.as_chunks::<3>().0 {
            let corners = triangle.map(|index| {
                let index = usize::try_from(index).expect("a u32 index fits in usize");
                self.vertices[index].pos
            });
            let normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalize_or_zero();
            if normal == Vec3::ZERO {
                continue;
            }
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                let (a, b) = (corners[a], corners[b]);
                let (key_a, key_b) = (key(a), key(b));
                if key_a == key_b {
                    continue;
                }
                let id = if key_a < key_b {
                    (key_a, key_b)
                } else {
                    (key_b, key_a)
                };
                edges
                    .entry(id)
                    .or_insert_with(|| Edge {
                        ends:    [a, b],
                        normals: vec![],
                    })
                    .normals
                    .push(normal);
            }
        }

        edges
            .into_values()
            .filter(|edge| match edge.normals.as_slice() {
                [first, second] => first.dot(*second) < crease,
                _ => true,
            })
            .map(|edge| edge.ends)
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        gm::{flat::Point, volume::Vertex3D},
        scene::MeshData,
    };

    fn vertex(position: Vec3) -> Vertex3D {
        Vertex3D::new(position, Vec3::Y, Point::default())
    }

    /// A cube of side 2 with flat faces, 4 vertices of its own per face.
    fn flat_cube() -> MeshData {
        let mut mesh = MeshData::default();
        for normal in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
            let u = normal.any_orthonormal_vector();
            let v = normal.cross(u);
            let first = u32::try_from(mesh.vertices.len()).expect("small");
            for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                mesh.vertices.push(vertex(normal + u * du + v * dv));
            }
            mesh.indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
        mesh
    }

    // A box has 12 edges, the diagonals of its faces are flat and give
    // none, and the 24 vertices of the flat faces weld into 8 corners.
    #[test]
    fn a_flat_cube_has_twelve_edges() {
        let edges = flat_cube().edges(30.0);
        assert_eq!(edges.len(), 12);
        for [a, b] in edges {
            assert!((a.distance(b) - 2.0).abs() < 1e-5, "{a} to {b} is no cube edge");
        }
    }

    // A lone quad has its 4 sides as a rim and no line across.
    #[test]
    fn an_open_quad_has_its_rim() {
        let mesh = MeshData {
            vertices: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, -1.0),
            ]
            .map(vertex)
            .to_vec(),
            indices:  vec![0, 1, 2, 0, 2, 3],
        };
        assert_eq!(mesh.edges(30.0).len(), 4);
    }

    // A crease of 100 degrees is more than the 90 of a box, so its
    // edges are smooth and only an open rim would show.
    #[test]
    fn a_crease_over_the_angle_hides_the_edge() {
        assert_eq!(flat_cube().edges(100.0), Vec::<[Vec3; 2]>::new());
    }
}
