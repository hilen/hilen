use std::sync::Arc;

use glam::IVec3;
use log::warn;
use rapier3d::{
    geometry::Array2,
    parry::shape::{HeightField, HeightFieldCellStatus, SharedShape, TriMeshFlags},
    prelude::ColliderBuilder,
};

use crate::{
    gm::{
        LossyConvert,
        flat::Point,
        volume::{Shape3, Vec3, Vertex3D},
    },
    scene::MeshData,
};

/// The solid a node collides with. By default it comes from the node's
/// drawn `Shape3`, `NodeTemplates::set_collider` puts another one on, so
/// a model can collide as a capsule and a terrain mesh as its heightfield.
#[derive(Clone, Debug)]
pub enum ColliderShape {
    /// Full side lengths.
    Box(Vec3),
    Ball(f32),
    /// Upright along y, `height` from tip to tip, caps included.
    Capsule {
        radius: f32,
        height: f32,
    },
    /// Upright along y.
    Cylinder {
        radius: f32,
        height: f32,
    },
    Heightfield(Arc<Heightfield>),
    /// The triangles of a mesh, for ground of any shape like a cave.
    /// The mesh must hold at least 1 triangle.
    TriMesh(Arc<MeshData>),
    /// Cubes of side `size` at the integer `cells`, cell `[0, 0, 0]`
    /// centered at `size / 2` along each axis from the node's origin.
    Voxels {
        size:  f32,
        cells: Arc<Vec<[i32; 3]>>,
    },
}

/// 2 triangle meshes are the same shape when they share 1 mesh, the
/// vertices are not compared.
impl PartialEq for ColliderShape {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Box(a), Self::Box(b)) => a == b,
            (Self::Ball(a), Self::Ball(b)) => a == b,
            (
                Self::Capsule { radius, height },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                },
            )
            | (
                Self::Cylinder { radius, height },
                Self::Cylinder {
                    radius: other_radius,
                    height: other_height,
                },
            ) => radius == other_radius && height == other_height,
            (Self::Heightfield(a), Self::Heightfield(b)) => a == b,
            (Self::TriMesh(a), Self::TriMesh(b)) => Arc::ptr_eq(a, b),
            (
                Self::Voxels { size, cells },
                Self::Voxels {
                    size: other_size,
                    cells: other_cells,
                },
            ) => size == other_size && cells == other_cells,
            _ => false,
        }
    }
}

impl ColliderShape {
    /// The collider a drawn shape gets, and where it sits from the node's
    /// origin, see `Shape3::collider_offset`.
    pub(crate) fn of(shape: Shape3) -> (Self, Vec3) {
        let solid = match shape {
            Shape3::Ball(radius) => Self::Ball(radius),
            Shape3::Box(_) | Shape3::Plane(_) | Shape3::Model(_) => Self::Box(shape.half_extents() * 2.0),
        };
        (solid, shape.collider_offset())
    }

    /// Half the size along each axis, the box the wireframe of a
    /// heightfield or a box fills.
    pub(crate) fn half_extents(&self) -> Vec3 {
        match self {
            Self::Box(size) => *size / 2.0,
            Self::Ball(radius) => Vec3::splat(*radius),
            Self::Capsule { radius, height } | Self::Cylinder { radius, height } => {
                Vec3::new(*radius, height / 2.0, *radius)
            }
            Self::Heightfield(field) => {
                let (low, high) = field.height_range();
                Vec3::new(field.width / 2.0, (high - low) / 2.0, field.depth / 2.0)
            }
            Self::TriMesh(_) | Self::Voxels { .. } => {
                let (low, high) = self.bounds();
                (high - low) / 2.0
            }
        }
    }

    /// The lowest and the highest corner of the box around the shape,
    /// from the node's origin.
    pub(crate) fn bounds(&self) -> (Vec3, Vec3) {
        match self {
            Self::TriMesh(mesh) => mesh.vertices.iter().fold(
                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                |(low, high), vertex| (low.min(vertex.pos), high.max(vertex.pos)),
            ),
            Self::Voxels { size, cells } => {
                let (low, high) = cells.iter().fold((IVec3::MAX, IVec3::MIN), |(low, high), &cell| {
                    (low.min(IVec3::from(cell)), high.max(IVec3::from(cell)))
                });
                (low.as_vec3() * *size, (high + IVec3::ONE).as_vec3() * *size)
            }
            _ => {
                let half = self.half_extents();
                (-half, half)
            }
        }
    }

    /// The rapier collider at a node's uniform `scale`.
    pub(crate) fn builder(&self, scale: f32) -> ColliderBuilder {
        match self {
            Self::Box(size) => {
                let half = *size / 2.0 * scale;
                ColliderBuilder::cuboid(half.x, half.y, half.z)
            }
            Self::Ball(radius) => ColliderBuilder::ball(radius * scale),
            Self::Capsule { radius, height } => {
                // Rapier measures the straight part between the caps.
                let straight = (height - radius * 2.0).max(0.0);
                ColliderBuilder::capsule_y(straight / 2.0 * scale, radius * scale)
            }
            Self::Cylinder { radius, height } => {
                ColliderBuilder::cylinder(height / 2.0 * scale, radius * scale)
            }
            Self::Heightfield(field) => {
                let size = Vec3::new(field.width * scale, scale, field.depth * scale);
                if field.holes.is_empty() {
                    return ColliderBuilder::heightfield(field.column_major(), size);
                }
                let mut shape = HeightField::new(field.column_major(), size);
                for row in 0..field.rows - 1 {
                    for column in 0..field.columns - 1 {
                        if field.is_hole(row, column) {
                            shape.set_cell_status(row, column, HeightFieldCellStatus::CELL_REMOVED);
                        }
                    }
                }
                ColliderBuilder::new(SharedShape::new(shape))
            }
            Self::TriMesh(mesh) => trimesh(mesh, scale),
            Self::Voxels { size, cells } => {
                let cells: Vec<IVec3> = cells.iter().map(|&cell| IVec3::from(cell)).collect();
                ColliderBuilder::voxels(Vec3::splat(size * scale), &cells)
            }
        }
    }
}

/// The triangles with their shared corners merged and the edges inside
/// a flat or smooth surface fixed, so a body sliding over the mesh does
/// not catch on them. A mesh whose topology rapier cannot read keeps
/// the edges as they are.
fn trimesh(mesh: &MeshData, scale: f32) -> ColliderBuilder {
    let vertices: Vec<Vec3> = mesh.vertices.iter().map(|vertex| vertex.pos * scale).collect();
    let indices: Vec<[u32; 3]> = mesh.indices.as_chunks::<3>().0.to_vec();
    assert!(
        !indices.is_empty(),
        "a triangle mesh collider needs at least 1 triangle"
    );
    let fixed = TriMeshFlags::MERGE_DUPLICATE_VERTICES | TriMeshFlags::FIX_INTERNAL_EDGES;
    match ColliderBuilder::trimesh_with_flags(vertices.clone(), indices.clone(), fixed) {
        Ok(builder) => builder,
        Err(err) => {
            warn!(
                "a triangle mesh collider of {} triangles keeps its inner edges: {err}",
                indices.len()
            );
            ColliderBuilder::trimesh_with_flags(vertices, indices, TriMeshFlags::MERGE_DUPLICATE_VERTICES)
                .expect("a triangle mesh with at least 1 triangle builds")
        }
    }
}

/// A grid of heights for ground, centered on the node's origin. Row by
/// row, a row runs along +x and the rows step along +z, so the height of
/// `row` and `column` is `heights[row * columns + column]`. The first
/// column sits at `-width / 2` and the last at `width / 2`, the rows
/// likewise across `depth`.
#[derive(Clone, Debug, PartialEq)]
pub struct Heightfield {
    pub heights: Vec<f32>,
    pub columns: usize,
    pub rows:    usize,
    pub width:   f32,
    pub depth:   f32,
    /// The cells with no ground, nothing collides there and the mesh has
    /// no triangles there. A cell spans from a grid point to the next row
    /// and column, cell `(row, column)` is at `row * (columns - 1) +
    /// column`. Empty when the field has no hole.
    pub holes:   Vec<bool>,
}

impl Heightfield {
    /// Fills the grid from `height(x, z)` sampled at every point.
    pub fn from_fn(
        columns: usize,
        rows: usize,
        width: f32,
        depth: f32,
        height: impl Fn(f32, f32) -> f32,
    ) -> Self {
        assert!(
            columns > 1 && rows > 1,
            "a heightfield needs at least 2 rows and 2 columns"
        );
        let mut field = Self {
            heights: vec![],
            columns,
            rows,
            width,
            depth,
            holes: vec![],
        };
        field.heights = (0..rows)
            .flat_map(|row| (0..columns).map(move |column| (row, column)))
            .map(|(row, column)| {
                let (x, z) = field.point(row, column);
                height(x, z)
            })
            .collect();
        field
    }

    /// Where a grid point lies on the ground, x then z.
    pub fn point(&self, row: usize, column: usize) -> (f32, f32) {
        let step = |i: usize, count: usize, size: f32| {
            size * (i.lossy_convert() / (count - 1).lossy_convert() - 0.5)
        };
        (
            step(column, self.columns, self.width),
            step(row, self.rows, self.depth),
        )
    }

    pub fn height(&self, row: usize, column: usize) -> f32 {
        self.heights[row * self.columns + column]
    }

    /// Takes the ground out of the cell from grid point `(row, column)` to
    /// the next row and column.
    pub fn cut_hole(&mut self, row: usize, column: usize) {
        assert!(
            row < self.rows - 1 && column < self.columns - 1,
            "cell {row} {column} is outside a field of {} by {} points",
            self.rows,
            self.columns
        );
        if self.holes.is_empty() {
            self.holes = vec![false; (self.rows - 1) * (self.columns - 1)];
        }
        self.holes[row * (self.columns - 1) + column] = true;
    }

    pub fn is_hole(&self, row: usize, column: usize) -> bool {
        !self.holes.is_empty() && self.holes[row * (self.columns - 1) + column]
    }

    /// The lowest and the highest height.
    pub fn height_range(&self) -> (f32, f32) {
        self.heights.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), &h| {
            (low.min(h), high.max(h))
        })
    }

    /// Rapier stores the grid a column at a time.
    fn column_major(&self) -> Array2<f32> {
        assert_eq!(
            self.heights.len(),
            self.columns * self.rows,
            "a heightfield holds rows * columns heights"
        );
        let data = (0..self.columns)
            .flat_map(|column| (0..self.rows).map(move |row| (row, column)))
            .map(|(row, column)| self.height(row, column))
            .collect();
        Array2::new(self.rows, self.columns, data)
    }

    /// The ground this heightfield collides as, a smooth mesh for a
    /// `Shape3::Model`. Every cell is cut along the same diagonal rapier
    /// cuts it, so what is drawn is exactly what bodies rest on.
    pub fn mesh(&self) -> MeshData {
        let at = |row: usize, column: usize| {
            let (x, z) = self.point(row, column);
            Vec3::new(x, self.height(row, column), z)
        };
        let vertices = (0..self.rows)
            .flat_map(|row| (0..self.columns).map(move |column| (row, column)))
            .map(|(row, column)| {
                // Central differences, one sided on the rim.
                let along_x = at(row, (column + 1).min(self.columns - 1)) - at(row, column.saturating_sub(1));
                let along_z = at((row + 1).min(self.rows - 1), column) - at(row.saturating_sub(1), column);
                let normal = along_z.cross(along_x).normalize();
                Vertex3D::new(at(row, column), normal, Point::default())
            })
            .collect();

        let columns = u32::try_from(self.columns).expect("a heightfield fits u32 indices");
        // Counter clockwise seen from above, the cut from the next row's
        // corner to the next column's, as rapier does it.
        let indices = (0..self.rows - 1)
            .flat_map(|row| (0..self.columns - 1).map(move |column| (row, column)))
            .filter(|&(row, column)| !self.is_hole(row, column))
            .map(|(row, column)| {
                u32::try_from(row * self.columns + column).expect("a heightfield fits u32 indices")
            })
            .flat_map(|i| [i, i + columns, i + 1, i + 1, i + columns, i + columns + 1])
            .collect();

        MeshData { vertices, indices }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn slope() -> Heightfield {
        Heightfield::from_fn(3, 4, 4.0, 6.0, |x, z| x + 10.0 * z)
    }

    #[test]
    fn points_span_the_size_centered() {
        let field = slope();
        assert_eq!(field.point(0, 0), (-2.0, -3.0));
        assert_eq!(field.point(3, 2), (2.0, 3.0));
        assert!((field.height(1, 2) - (2.0 - 10.0)).abs() < 1e-5);
        assert_eq!(field.height_range(), (-32.0, 32.0));
    }

    // Rapier reads the grid a column at a time with rows along z. A row
    // major grid handed over as is would lay the terrain out transposed,
    // and bodies would rest on hills that are not drawn.
    #[test]
    fn rapier_gets_the_grid_column_major() {
        let field = slope();
        let data = field.column_major();
        assert_eq!(data.nrows(), 4);
        assert_eq!(data.ncols(), 3);
        for row in 0..4 {
            for column in 0..3 {
                assert_eq!(data[(row, column)].to_bits(), field.height(row, column).to_bits());
            }
        }
    }

    #[test]
    fn the_mesh_faces_up_on_the_grid_points() {
        let field = Heightfield::from_fn(5, 5, 4.0, 4.0, |_, _| 1.5);
        let mesh = field.mesh();
        assert_eq!(mesh.vertices.len(), 25);
        assert_eq!(mesh.indices.len(), 4 * 4 * 6);
        assert!(
            mesh.vertices
                .iter()
                .all(|v| (v.pos.y - 1.5).abs() < 1e-6 && v.normal.distance(Vec3::Y) < 1e-6)
        );
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = triangle.map(|i| mesh.vertices[usize::try_from(i).unwrap()].pos);
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }

    #[test]
    fn a_hole_drops_its_triangles_and_its_ground() {
        let mut field = Heightfield::from_fn(5, 5, 4.0, 4.0, |_, _| 0.0);
        field.cut_hole(1, 2);
        assert!(field.is_hole(1, 2));
        assert!(!field.is_hole(2, 1));
        assert_eq!(field.mesh().indices.len(), (16 - 1) * 6);

        let ground = ColliderShape::Heightfield(Arc::new(field.clone())).builder(1.0).build();
        let down = rapier3d::parry::query::Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y);
        let hits = |x: f32, z: f32| {
            let ray = rapier3d::parry::query::Ray::new(Vec3::new(x, 5.0, z), down.dir);
            ground.shape().cast_ray(ground.position(), &ray, 100.0, true).is_some()
        };
        // Cell (1, 2) spans x 0 to 1 and z -1 to 0.
        let (x, z) = field.point(1, 2);
        assert!(!hits(x + 0.5, z + 0.5));
        assert!(hits(x - 0.5, z + 0.5));
        assert!(hits(x + 0.5, z + 1.5));
    }

    fn cube() -> MeshData {
        let corner = |x: f32, y: f32, z: f32| Vertex3D::new(Vec3::new(x, y, z), Vec3::Y, Point::default());
        let vertices = [
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.0, 1.0, 1.0),
        ]
        .map(|(x, y, z)| corner(x, y, z))
        .to_vec();
        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7, 3, 3, 7,
            4, 3, 4, 0,
        ];
        MeshData { vertices, indices }
    }

    #[test]
    fn a_triangle_mesh_collides_where_its_triangles_are() {
        let shape = ColliderShape::TriMesh(Arc::new(cube()));
        let (low, high) = shape.bounds();
        assert_eq!((low, high), (Vec3::ZERO, Vec3::ONE));
        let collider = shape.builder(2.0).build();
        let ray = rapier3d::parry::query::Ray::new(Vec3::new(1.0, 10.0, 1.0), Vec3::NEG_Y);
        let distance = collider
            .shape()
            .cast_ray(collider.position(), &ray, 100.0, true)
            .expect("hits the top");
        assert!((distance - 8.0).abs() < 1e-4);
    }

    #[test]
    fn voxels_fill_their_cells() {
        let shape = ColliderShape::Voxels {
            size:  0.5,
            cells: Arc::new(vec![[0, 0, 0], [1, 0, 0], [3, 2, 1]]),
        };
        assert_eq!(shape.bounds(), (Vec3::ZERO, Vec3::new(2.0, 1.5, 1.0)));
        let collider = shape.builder(1.0).build();
        let down = |x: f32, z: f32| {
            let ray = rapier3d::parry::query::Ray::new(Vec3::new(x, 10.0, z), Vec3::NEG_Y);
            collider.shape().cast_ray(collider.position(), &ray, 100.0, true)
        };
        assert!((down(0.75, 0.25).expect("cell 1 0 0") - 9.5).abs() < 1e-4);
        assert!((down(1.75, 0.75).expect("cell 3 2 1") - 8.5).abs() < 1e-4);
        assert!(down(1.25, 0.25).is_none());
    }

    #[test]
    fn a_capsule_hands_rapier_its_straight_part() {
        let capsule = ColliderShape::Capsule {
            radius: 0.5,
            height: 3.0,
        }
        .builder(2.0)
        .build();
        let shape = capsule.shape().as_capsule().expect("a capsule");
        assert!((shape.radius - 1.0).abs() < 1e-6);
        assert!((shape.half_height() - 2.0).abs() < 1e-6);
    }
}
