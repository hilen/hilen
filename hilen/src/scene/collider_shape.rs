use std::sync::Arc;

use rapier3d::{geometry::Array2, prelude::ColliderBuilder};

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
#[derive(Clone, Debug, PartialEq)]
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
            Self::Heightfield(field) => ColliderBuilder::heightfield(
                field.column_major(),
                Vec3::new(field.width * scale, scale, field.depth * scale),
            ),
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
        let rows = u32::try_from(self.rows).expect("a heightfield fits u32 indices");
        // Counter clockwise seen from above, the cut from the next row's
        // corner to the next column's, as rapier does it.
        let indices = (0..rows - 1)
            .flat_map(|row| (0..columns - 1).map(move |column| row * columns + column))
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
