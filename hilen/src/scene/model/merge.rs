//! Joins the parts of a static model that draw the same, so a scene
//! draws 1 mesh for each look and not 1 for each glTF primitive. A level
//! made of thousands of primitives took a draw, a bind and an instance
//! for each of them every frame.

use std::{collections::HashMap, mem::take};

use super::{
    material_source::MaterialKey,
    parse::{ModelSource, PartSource},
    split::split_u16,
    triangles,
};
use crate::gm::volume::{Mat3, Mat4, Vertex3D};

/// What a part draws with. Parts with the same look draw as 1 mesh.
type Look = (Option<MaterialKey>, bool, bool);

/// Joins the parts with the same look, each at its place in the model,
/// in the order the first part of each look comes. A model with a rig
/// keeps its parts, a clip moves them 1 by 1. Translucent parts of 2
/// looks draw in the order of their looks after the join, not in the
/// order of the file. Any thread.
pub(crate) fn static_parts(mut source: ModelSource) -> ModelSource {
    if source.rig.is_some() || source.parts.len() < 2 {
        return source;
    }
    if source.picking {
        source.triangles = Some(triangles::of_parts(&source.parts));
    }
    source.parts = joined(take(&mut source.parts));
    source
}

fn joined(parts: Vec<PartSource>) -> Vec<PartSource> {
    let mut groups: Vec<Vec<PartSource>> = vec![];
    let mut slots: HashMap<Look, usize> = HashMap::new();
    for part in parts {
        let look = (
            part.material.map(|material| material.key()),
            part.smooth_colors,
            part.flowing,
        );
        let slot = *slots.entry(look).or_insert_with(|| {
            groups.push(vec![]);
            groups.len() - 1
        });
        groups[slot].push(part);
    }
    groups.into_iter().flat_map(join).collect()
}

/// 1 group as parts of at most `MAX_VERTICES` vertices, every vertex at
/// its place in the model.
fn join(group: Vec<PartSource>) -> Vec<PartSource> {
    if group.len() == 1 {
        return group;
    }
    let first = &group[0];
    let (material, smooth_colors, flowing, node) =
        (first.material, first.smooth_colors, first.flowing, first.node);

    let mut vertices: Vec<Vertex3D> = Vec::with_capacity(group.iter().map(|part| part.vertices.len()).sum());
    let mut indices: Vec<usize> = Vec::with_capacity(group.iter().map(|part| part.indices.len()).sum());
    for part in &group {
        let base = vertices.len();
        indices.extend(part.indices.iter().map(|&index| base + usize::from(index)));
        if part.transform == Mat4::IDENTITY {
            vertices.extend_from_slice(&part.vertices);
            continue;
        }
        // The loader already swapped the corners of a part under a mirror,
        // so the vertices take the place alone and keep their order.
        let normals = Mat3::from_mat4(part.transform).inverse().transpose();
        vertices.extend(part.vertices.iter().map(|vertex| Vertex3D {
            pos: part.transform.transform_point3(vertex.pos),
            normal: (normals * vertex.normal).normalize_or_zero(),
            ..*vertex
        }));
    }

    split_u16(vertices.len(), &indices)
        .into_iter()
        .map(|(sources, indices)| PartSource {
            vertices: sources.iter().map(|&i| vertices[i]).collect(),
            skin_vertices: None,
            indices,
            transform: Mat4::IDENTITY,
            node,
            skin: None,
            material,
            smooth_colors,
            flowing,
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::{
        gm::{
            color::Color,
            flat::Point,
            volume::{Bounds, Quat, Vec3},
        },
        scene::model::{material_source::MaterialSource, parse::parse_glb, split::MAX_VERTICES},
    };

    fn material(texture: usize) -> MaterialSource {
        MaterialSource {
            color:              Color::rgba(1.0, 1.0, 1.0, 1.0),
            metallic:           0.0,
            roughness:          1.0,
            texture:            Some(texture),
            metallic_roughness: None,
            normal_map:         None,
            normal_scale:       1.0,
            occlusion:          None,
            occlusion_strength: 1.0,
            cutout:             None,
            unlit:              false,
        }
    }

    fn part(material: Option<MaterialSource>, transform: Mat4) -> PartSource {
        let vertex = |x: f32, y: f32| Vertex3D::new(Vec3::new(x, y, 0.0), Vec3::Z, Point::default());
        PartSource {
            vertices: vec![vertex(0.0, 0.0), vertex(1.0, 0.0), vertex(0.0, 1.0)],
            skin_vertices: None,
            indices: vec![0, 1, 2],
            transform,
            node: 0,
            skin: None,
            material,
            smooth_colors: false,
            flowing: false,
        }
    }

    fn source(parts: Vec<PartSource>) -> ModelSource {
        ModelSource {
            parts,
            images: vec![],
            bounds: Bounds::default(),
            rig: None,
            rest_joints: vec![],
            picking: true,
            triangles: None,
        }
    }

    fn corners(part: &PartSource) -> Vec<[Vec3; 3]> {
        triangles::of_parts(std::slice::from_ref(part))
    }

    fn fixture(name: &str) -> ModelSource {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models").join(name);
        parse_glb(&fs::read(&path).unwrap(), name).unwrap()
    }

    #[test]
    fn parts_of_one_look_join_at_their_places() {
        let moved = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        // A mirror, the loader swapped its corners already.
        let mirrored = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let parts = vec![
            part(Some(material(0)), Mat4::IDENTITY),
            part(Some(material(0)), moved),
            part(Some(material(0)), mirrored),
        ];
        let before: Vec<[Vec3; 3]> = parts.iter().flat_map(corners).collect();

        let merged = static_parts(source(parts));

        assert_eq!(merged.parts.len(), 1);
        let joined = &merged.parts[0];
        assert_eq!(joined.transform, Mat4::IDENTITY);
        assert_eq!(
            corners(joined),
            before,
            "every corner where it was drawn, in its order"
        );
        assert_eq!(
            merged.triangles,
            Some(before),
            "picking keeps the triangles in file order"
        );
    }

    #[test]
    fn a_turned_part_turns_its_normals() {
        let turned = Mat4::from_quat(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let merged = static_parts(source(vec![part(None, Mat4::IDENTITY), part(None, turned)]));
        let normals: Vec<Vec3> = merged.parts[0].vertices.iter().map(|vertex| vertex.normal).collect();
        assert!(normals[..3].iter().all(|normal| normal.abs_diff_eq(Vec3::Z, 1e-6)));
        assert!(
            normals[3..].iter().all(|normal| normal.abs_diff_eq(Vec3::X, 1e-6)),
            "{normals:?}"
        );
    }

    #[test]
    fn parts_of_2_looks_stay_apart_in_the_order_they_come() {
        let parts = vec![
            part(Some(material(1)), Mat4::IDENTITY),
            part(Some(material(0)), Mat4::IDENTITY),
            part(Some(material(1)), Mat4::IDENTITY),
            part(None, Mat4::IDENTITY),
        ];
        let merged = static_parts(source(parts));
        let textures: Vec<Option<usize>> = merged
            .parts
            .iter()
            .map(|part| part.material.and_then(|material| material.texture))
            .collect();
        assert_eq!(textures, vec![Some(1), Some(0), None]);
        assert_eq!(merged.parts[0].indices.len(), 6);
    }

    #[test]
    fn a_join_over_the_16_bit_limit_splits() {
        let count = MAX_VERTICES / 3 + 1;
        let parts = (0..count).map(|_| part(Some(material(0)), Mat4::IDENTITY)).collect();
        let merged = static_parts(source(parts));
        assert_eq!(merged.parts.len(), 2);
        assert!(merged.parts.iter().all(|part| part.vertices.len() <= MAX_VERTICES));
        let triangles: usize = merged.parts.iter().map(|part| part.indices.len() / 3).sum();
        assert_eq!(triangles, count);
    }

    #[test]
    fn a_model_with_a_rig_keeps_its_parts() {
        let windmill = fixture("windmill.glb");
        let count = windmill.parts.len();
        assert!(count > 1);
        assert_eq!(static_parts(windmill).parts.len(), count);
    }

    #[test]
    fn the_tree_draws_fewer_parts_and_picks_the_same_triangles() {
        let tree = fixture("tree.glb");
        let before = triangles::of_parts(&tree.parts);
        let mut looks: Vec<Option<MaterialKey>> = vec![];
        for part in &tree.parts {
            let look = part.material.map(|material| material.key());
            if !looks.contains(&look) {
                looks.push(look);
            }
        }
        let expected: Vec<[Vec3; 3]> = looks
            .iter()
            .flat_map(|look| {
                tree.parts
                    .iter()
                    .filter(move |part| part.material.map(|material| material.key()) == *look)
                    .flat_map(corners)
            })
            .collect();
        let parts = tree.parts.len();

        let merged = static_parts(tree);

        assert!(merged.parts.len() < parts, "{} of {parts}", merged.parts.len());
        assert_eq!(merged.triangles.as_ref(), Some(&before));
        let after = triangles::of_parts(&merged.parts);
        assert_eq!(after.len(), expected.len());
        for (a, b) in after.iter().zip(&expected) {
            for (x, y) in a.iter().zip(b) {
                assert!(x.abs_diff_eq(*y, 1e-4), "{x} {y}");
            }
        }
    }
}
