use super::{Model, parse::PartSource};
use crate::gm::volume::{Ray, Vec3};

/// A determinant this small is a ray along the plane of the triangle.
const PARALLEL: f32 = 1e-9;

/// Every triangle of the parts in model space, each part at its place at
/// rest, in the order of the parts. A mesh built in code is split into
/// parts in the order of its triangles, so its triangle `n` stays `n`.
/// A skinned part is taken at its bind pose.
pub(super) fn of_parts(parts: &[PartSource]) -> Vec<[Vec3; 3]> {
    parts
        .iter()
        .flat_map(|part| {
            part.indices.as_chunks::<3>().0.iter().map(|triangle| {
                triangle.map(|index| part.transform.transform_point3(part.vertices[usize::from(index)].pos))
            })
        })
        .collect()
}

impl Model {
    /// How many triangles the model has, see `triangle`.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// The corners of triangle `index` in model space at rest. For a mesh
    /// built in code it is the triangle of indices `3 * index` to
    /// `3 * index + 2` of its `MeshData`, so an app reads its own data per
    /// triangle by it, like the part a CSG library made it from.
    pub fn triangle(&self, index: usize) -> [Vec3; 3] {
        self.triangles[index]
    }

    /// The nearest triangle `ray` meets from its front, the side its
    /// corners go counter clockwise, the side that is drawn, with the
    /// distance along the ray. The ray is in model space and its direction
    /// need not be a unit, the distance counts in its length.
    pub fn hit_triangle(&self, ray: Ray) -> Option<(f32, usize)> {
        self.triangles
            .iter()
            .enumerate()
            .filter_map(|(index, triangle)| hit(ray, *triangle).map(|distance| (distance, index)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}

/// The Moller Trumbore test, front faces only, hits from the ray's start
/// on.
fn hit(ray: Ray, [first, second, third]: [Vec3; 3]) -> Option<f32> {
    let to_second = second - first;
    let to_third = third - first;
    let across = ray.direction.cross(to_third);
    let determinant = to_second.dot(across);
    if determinant < PARALLEL {
        return None;
    }
    let to_origin = ray.origin - first;
    let along_second = to_origin.dot(across) / determinant;
    if !(0.0..=1.0).contains(&along_second) {
        return None;
    }
    let up = to_origin.cross(to_second);
    let along_third = ray.direction.dot(up) / determinant;
    if along_third < 0.0 || along_second + along_third > 1.0 {
        return None;
    }
    let distance = to_third.dot(up) / determinant;
    (distance >= 0.0).then_some(distance)
}

#[cfg(test)]
mod test {
    use super::*;

    const TRIANGLE: [Vec3; 3] = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];

    fn toward(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction }
    }

    #[test]
    fn a_ray_meets_the_front_inside_and_misses_outside() {
        let front = hit(toward(Vec3::new(0.25, 0.25, 5.0), Vec3::NEG_Z), TRIANGLE);
        assert!((front.expect("inside") - 5.0).abs() < 1e-6);
        assert_eq!(hit(toward(Vec3::new(0.8, 0.8, 5.0), Vec3::NEG_Z), TRIANGLE), None);
        assert_eq!(
            hit(toward(Vec3::new(-0.1, 0.5, 5.0), Vec3::NEG_Z), TRIANGLE),
            None
        );
    }

    // A back face is not drawn, so a ray from behind goes through it, and
    // a triangle behind the start is not met.
    #[test]
    fn the_back_and_behind_are_missed() {
        assert_eq!(hit(toward(Vec3::new(0.25, 0.25, -5.0), Vec3::Z), TRIANGLE), None);
        assert_eq!(hit(toward(Vec3::new(0.25, 0.25, 5.0), Vec3::Z), TRIANGLE), None);
    }

    // The distance counts in the length of the direction, so a ray put
    // into the space of a scaled node gives the distance of the world.
    #[test]
    fn the_distance_counts_in_the_length_of_the_direction() {
        let half = hit(
            toward(Vec3::new(0.25, 0.25, 5.0), Vec3::new(0.0, 0.0, -2.0)),
            TRIANGLE,
        );
        assert!((half.expect("inside") - 2.5).abs() < 1e-6);
    }

    #[test]
    fn a_mesh_from_code_keeps_its_triangle_order() {
        use crate::{
            gm::{flat::Point, volume::Vertex3D},
            scene::{MeshData, model::code_mesh::mesh_source},
        };

        let vertex = |x: f32, y: f32, z: f32| Vertex3D::new(Vec3::new(x, y, z), Vec3::Z, Point::default());
        let mesh = MeshData {
            vertices: vec![
                vertex(0.0, 0.0, 0.0),
                vertex(1.0, 0.0, 0.0),
                vertex(0.0, 1.0, 0.0),
                vertex(5.0, 0.0, -1.0),
                vertex(6.0, 0.0, -1.0),
                vertex(5.0, 1.0, -1.0),
            ],
            indices:  vec![0, 1, 2, 3, 4, 5],
        };
        let triangles = of_parts(&mesh_source(mesh).parts);
        assert_eq!(triangles.len(), 2);
        assert_eq!(triangles[1][0], Vec3::new(5.0, 0.0, -1.0));
    }
}
