use std::convert::Infallible;

use super::{
    Model,
    parse::{ModelSource, PartSource},
    split::split_u16,
};
use crate::{
    deps::refs::{Weak, manage::DataManager},
    gm::volume::{Bounds, Mat4, Vertex3D},
};

/// Geometry built in code, for a mesh no `.glb` holds, like a terrain
/// or a procedural prop. Every three `indices` are one triangle, wound
/// counter clockwise seen from outside, since back faces are culled.
/// Any vertex count, `Model::from_mesh` splits a big one for the 16 bit
/// indices every lane draws.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex3D>,
    pub indices:  Vec<u32>,
}

impl Model {
    /// A model of geometry built in code, stored under `name` like a
    /// loaded file, so `Model::get(name)` returns it too and every node
    /// of it draws the same buffers. When `name` is already stored that
    /// model is returned and `mesh` is dropped, free the old one first to
    /// replace it. It has no material, so a node's own material draws it,
    /// and its collider and picking box is the box around its vertices.
    /// Call it on the main thread, it uploads to the GPU.
    pub fn from_mesh(name: &str, mesh: MeshData) -> Weak<Model> {
        let Ok(model) = Self::store_with_name::<Infallible>(name, || Ok(Self::upload(mesh_source(mesh))));
        model
    }
}

fn mesh_source(mesh: MeshData) -> ModelSource {
    assert_eq!(
        mesh.indices.len() % 3,
        0,
        "a mesh is whole triangles, 3 indices each"
    );
    let indices: Vec<usize> = mesh
        .indices
        .iter()
        .map(|&i| usize::try_from(i).expect("a u32 index fits in usize"))
        .collect();
    assert!(
        indices.iter().all(|&i| i < mesh.vertices.len()),
        "a mesh index points past its {} vertices",
        mesh.vertices.len()
    );
    // The color reaches the fragment flat from a corner the backend picks,
    // so corners that differ would draw a different color on each lane.
    assert!(
        indices.as_chunks::<3>().0.iter().all(|&[a, b, c]| {
            let color = mesh.vertices[a].color;
            mesh.vertices[b].color == color && mesh.vertices[c].color == color
        }),
        "the three corners of a triangle need one color"
    );

    let parts = split_u16(mesh.vertices.len(), &indices)
        .into_iter()
        .map(|(sources, indices)| PartSource {
            vertices: sources.iter().map(|&i| mesh.vertices[i]).collect(),
            skin_vertices: None,
            indices,
            transform: Mat4::IDENTITY,
            node: 0,
            skin: None,
            material: None,
        })
        .collect();

    ModelSource {
        parts,
        images: vec![],
        bounds: Bounds::of_points(mesh.vertices.iter().map(|vertex| vertex.pos)),
        rig: None,
        rest_joints: vec![],
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gm::{color::U8Color, flat::Point, volume::Vec3};

    fn vertex(x: f32, y: f32, z: f32) -> Vertex3D {
        Vertex3D::new(Vec3::new(x, y, z), Vec3::Y, Point::default())
    }

    #[test]
    fn a_small_mesh_is_one_part_bounded_by_its_vertices() {
        let source = mesh_source(MeshData {
            vertices: vec![
                vertex(-1.0, 0.0, 2.0),
                vertex(3.0, 0.5, 2.0),
                vertex(0.0, -2.0, -4.0),
            ],
            indices:  vec![0, 2, 1],
        });
        assert_eq!(source.parts.len(), 1);
        assert_eq!(source.parts[0].indices, vec![0, 2, 1]);
        assert!(source.parts[0].material.is_none());
        assert_eq!(source.bounds.min, Vec3::new(-1.0, -2.0, -4.0));
        assert_eq!(source.bounds.max, Vec3::new(3.0, 0.5, 2.0));
    }

    // A terrain grid is bigger than one 16 bit mesh. Every triangle has to
    // land in some part with its own three vertices, else the ground has
    // holes along the seams.
    #[test]
    fn a_big_grid_splits_and_keeps_every_triangle() {
        let side = 300u32;
        let vertices: Vec<Vertex3D> = (0..side * side)
            .map(|i| {
                vertex(
                    f32::from(u16::try_from(i % side).unwrap()),
                    0.0,
                    f32::from(u16::try_from(i / side).unwrap()),
                )
            })
            .collect();
        let indices: Vec<u32> = (0..side - 1)
            .flat_map(|z| (0..side - 1).map(move |x| z * side + x))
            .flat_map(|i| [i, i + side, i + 1, i + 1, i + side, i + side + 1])
            .collect();
        let triangles = indices.len() / 3;

        let source = mesh_source(MeshData {
            vertices: vertices.clone(),
            indices,
        });

        assert!(source.parts.len() > 1);
        let drawn: usize = source.parts.iter().map(|part| part.indices.len() / 3).sum();
        assert_eq!(drawn, triangles);
        for part in &source.parts {
            assert!(part.indices.iter().all(|&i| usize::from(i) < part.vertices.len()));
        }
        assert_eq!(source.bounds.max, Vec3::new(299.0, 0.0, 299.0));
    }

    #[test]
    #[should_panic(expected = "one color")]
    fn a_triangle_with_two_colors_panics() {
        let mut red = vertex(1.0, 0.0, 0.0);
        red.color = U8Color::const_rgb(255, 0, 0);
        mesh_source(MeshData {
            vertices: vec![vertex(0.0, 0.0, 0.0), red, vertex(0.0, 0.0, 1.0)],
            indices:  vec![0, 2, 1],
        });
    }

    #[test]
    #[should_panic(expected = "points past")]
    fn an_index_past_the_vertices_panics() {
        mesh_source(MeshData {
            vertices: vec![vertex(0.0, 0.0, 0.0)],
            indices:  vec![0, 0, 1],
        });
    }
}
