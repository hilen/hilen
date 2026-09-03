use std::mem::take;

use anyhow::{Result, anyhow, bail, ensure};
use gltf::{
    Document, Gltf, Node, Primitive,
    animation::{Interpolation as KeyInterpolation, Property, util::ReadOutputs},
    buffer, image,
    material::AlphaMode,
    mesh::{
        Mode,
        util::{ReadJoints, ReadWeights},
    },
};
use log::warn;

use super::rig::{Channel, Clip, Interpolation, Rig, RigNode, Skin, Track};
use crate::gm::{
    color::Color,
    flat::Point,
    volume::{Bounds, Mat4, Quat, SkinVertex, Vec3, Vertex3D},
};

/// Indices are 16 bit, so a mesh holds at most this many vertices and a
/// bigger primitive is split.
const MAX_VERTICES: usize = u16::MAX as usize;

/// Everything a `.glb` holds, decoded on the CPU and ready to upload.
pub(crate) struct ModelSource {
    pub parts:       Vec<PartSource>,
    pub images:      Vec<EmbeddedImage>,
    pub bounds:      Bounds,
    /// The node tree with its skins and clips, only for a file that has
    /// either.
    pub rig:         Option<Rig>,
    /// Every skin's joint matrices at rest, in the rig's skin order.
    pub rest_joints: Vec<Vec<Mat4>>,
}

/// One primitive of one node, or a slice of a primitive too big for 16
/// bit indices.
pub(crate) struct PartSource {
    pub vertices:      Vec<Vertex3D>,
    /// One per vertex when the primitive is skinned.
    pub skin_vertices: Option<Vec<SkinVertex>>,
    pub indices:       Vec<u16>,
    /// The node's place in the model at rest, every parent applied.
    /// Identity for a skinned part, its joints place it.
    pub transform:     Mat4,
    /// The node this part belongs to, what a clip moves.
    pub node:          usize,
    /// The skin over this part, an index into the rig's skins.
    pub skin:          Option<usize>,
    /// None when the primitive has no material, the node's own applies.
    pub material:      Option<MaterialSource>,
}

/// A glTF material, its textures as indices into `ModelSource::images`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialSource {
    pub color:        Color,
    pub metallic:     f32,
    pub roughness:    f32,
    pub texture:      Option<usize>,
    pub normal_map:   Option<usize>,
    pub normal_scale: f32,
}

/// An image file embedded in the binary chunk, png or jpeg bytes.
pub(crate) struct EmbeddedImage {
    pub name:  String,
    pub bytes: Vec<u8>,
}

/// The vertex arrays of one primitive before the 16 bit split.
struct Geometry {
    vertices: Vec<Vertex3D>,
    skin:     Option<Vec<SkinVertex>>,
    indices:  Vec<usize>,
}

pub(crate) fn parse_glb(data: &[u8], name: &str) -> Result<ModelSource> {
    let gltf = Gltf::from_slice(data)?;
    let document = &gltf.document;

    for buffer in document.buffers() {
        if let buffer::Source::Uri(uri) = buffer.source() {
            bail!(
                "{name}: buffer {} is the external file {uri}, only a .glb with embedded buffers loads",
                buffer.index()
            );
        }
    }
    let blob = gltf
        .blob
        .as_deref()
        .ok_or_else(|| anyhow!("{name}: no binary chunk, only a .glb with embedded buffers loads"))?;

    let images = document
        .images()
        .map(|image| match image.source() {
            image::Source::View { view, .. } => Ok(EmbeddedImage {
                name:  format!("{name}#{}", image.index()),
                bytes: blob[view.offset()..view.offset() + view.length()].to_vec(),
            }),
            image::Source::Uri { uri, .. } => Err(anyhow!(
                "{name}: image {} is the external file {uri}, only embedded images load",
                image.index()
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or_else(|| anyhow!("{name}: the file has no scene"))?;

    let rig = rig(document, blob, name)?;

    let mut parts = vec![];
    for node in scene.nodes() {
        walk(&node, Mat4::IDENTITY, blob, name, rig.as_ref(), &mut parts)?;
    }

    let rest_joints = rig.as_ref().map_or_else(Vec::new, |rig| {
        let globals = rig.pose(None);
        rig.skins.iter().map(|skin| skin.joint_matrices(&globals)).collect()
    });

    let bounds = Bounds::of_points(parts.iter().flat_map(|part| {
        let joints = part.skin.map(|skin| &rest_joints[skin]);
        part.vertices
            .iter()
            .enumerate()
            .map(move |(i, vertex)| match (joints, &part.skin_vertices) {
                (Some(joints), Some(skin)) => skinned_point(vertex.pos, &skin[i], joints),
                _ => part.transform.transform_point3(vertex.pos),
            })
    }));

    Ok(ModelSource {
        parts,
        images,
        bounds,
        rig,
        rest_joints,
    })
}

/// Where the joints put a vertex, the weighted blend of the four.
fn skinned_point(pos: Vec3, skin: &SkinVertex, joints: &[Mat4]) -> Vec3 {
    skin.joints
        .iter()
        .zip(skin.weights)
        .map(|(&joint, weight)| joints[usize::from(joint)].transform_point3(pos) * weight)
        .sum()
}

fn walk(
    node: &Node,
    parent: Mat4,
    blob: &[u8],
    name: &str,
    rig: Option<&Rig>,
    parts: &mut Vec<PartSource>,
) -> Result<()> {
    let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());

    if let Some(mesh) = node.mesh() {
        // The skin index and how many joints it has.
        let skin = node.skin().map(|skin| {
            let index = skin.index();
            (
                index,
                rig.expect("a skinned node has a rig").skins[index].joints.len(),
            )
        });
        for primitive in mesh.primitives() {
            parts.extend(primitive_parts(
                &primitive,
                transform,
                node.index(),
                skin,
                blob,
                name,
            )?);
        }
    }

    for child in node.children() {
        walk(&child, transform, blob, name, rig, parts)?;
    }

    Ok(())
}

fn primitive_parts(
    primitive: &Primitive,
    transform: Mat4,
    node: usize,
    skin: Option<(usize, usize)>,
    blob: &[u8],
    name: &str,
) -> Result<Vec<PartSource>> {
    ensure!(
        primitive.mode() == Mode::Triangles,
        "{name}: a primitive is drawn as {:?}, only triangles load",
        primitive.mode()
    );

    // A .glb has one binary buffer and every view points into it.
    let reader = primitive.reader(|buffer| (buffer.index() == 0).then_some(blob));

    let positions: Vec<Vec3> = reader
        .read_positions()
        .ok_or_else(|| anyhow!("{name}: a primitive has no positions"))?
        .map(Vec3::from)
        .collect();
    let normals: Option<Vec<Vec3>> = reader.read_normals().map(|normals| normals.map(Vec3::from).collect());
    let uvs: Option<Vec<Point>> = reader
        .read_tex_coords(0)
        .map(|uvs| uvs.into_f32().map(|[u, v]| Point::new(u, v)).collect());

    let mut indices: Vec<usize> = match reader.read_indices() {
        Some(indices) => indices.into_u32().map(|index| index as usize).collect(),
        None => (0..positions.len()).collect(),
    };

    ensure!(
        indices.len().is_multiple_of(3),
        "{name}: a primitive has {} indices, not triangles",
        indices.len()
    );
    ensure!(
        indices.iter().all(|index| *index < positions.len()),
        "{name}: a primitive indexes past its {} vertices",
        positions.len()
    );
    if let Some(normals) = &normals {
        ensure!(
            normals.len() == positions.len(),
            "{name}: a primitive has {} normals for {} positions",
            normals.len(),
            positions.len()
        );
    }
    if let Some(uvs) = &uvs {
        ensure!(
            uvs.len() == positions.len(),
            "{name}: a primitive has {} uvs for {} positions",
            uvs.len(),
            positions.len()
        );
    }

    let skin_vertices = match skin {
        Some((_, joints)) => Some(skin_vertices(
            reader.read_joints(0),
            reader.read_weights(0),
            positions.len(),
            joints,
            name,
        )?),
        None => None,
    };

    // A mirroring transform turns the winding inside out, and back faces
    // are culled.
    if transform.determinant() < 0.0 {
        for triangle in indices.as_chunks_mut::<3>().0 {
            triangle.swap(1, 2);
        }
    }

    let geometry = match normals {
        Some(normals) => Geometry {
            vertices: positions
                .iter()
                .zip(&normals)
                .enumerate()
                .map(|(i, (pos, normal))| Vertex3D {
                    pos:    *pos,
                    normal: *normal,
                    uv:     uvs.as_ref().map_or_else(Point::default, |uvs| uvs[i]),
                })
                .collect(),
            skin: skin_vertices,
            indices,
        },
        None => flat_shaded(&positions, uvs.as_deref(), skin_vertices.as_deref(), &indices),
    };

    let material = material_source(&primitive.material());

    // The skinned mesh's own node transform is ignored, its joints carry
    // the whole placement.
    let transform = if skin.is_some() { Mat4::IDENTITY } else { transform };

    Ok(split_u16(geometry.vertices.len(), &geometry.indices)
        .into_iter()
        .map(|(sources, indices)| PartSource {
            vertices: sources.iter().map(|&i| geometry.vertices[i]).collect(),
            skin_vertices: geometry.skin.as_ref().map(|skin| sources.iter().map(|&i| skin[i]).collect()),
            indices,
            transform,
            node,
            skin: skin.map(|(index, _)| index),
            material,
        })
        .collect())
}

/// The joints and weights of a skinned primitive, one per position,
/// every joint inside the skin's `joint_count`.
fn skin_vertices(
    joints: Option<ReadJoints>,
    weights: Option<ReadWeights>,
    positions: usize,
    joint_count: usize,
    name: &str,
) -> Result<Vec<SkinVertex>> {
    let joints = joints
        .ok_or_else(|| anyhow!("{name}: a skinned primitive has no joints"))?
        .into_u16();
    let weights = weights
        .ok_or_else(|| anyhow!("{name}: a skinned primitive has no weights"))?
        .into_f32();
    let vertices: Vec<SkinVertex> = joints
        .zip(weights)
        .map(|(joints, weights)| SkinVertex { joints, weights })
        .collect();
    ensure!(
        vertices.len() == positions,
        "{name}: a primitive has {} skin vertices for {positions} positions",
        vertices.len()
    );
    ensure!(
        vertices
            .iter()
            .all(|vertex| vertex.joints.iter().all(|&joint| usize::from(joint) < joint_count)),
        "{name}: a vertex names a joint past the {joint_count} of its skin"
    );
    Ok(vertices)
}

/// Without normals glTF asks for flat shading, so every triangle gets
/// its own three vertices carrying the face normal.
fn flat_shaded(
    positions: &[Vec3],
    uvs: Option<&[Point]>,
    skin: Option<&[SkinVertex]>,
    indices: &[usize],
) -> Geometry {
    let mut vertices = Vec::with_capacity(indices.len());
    let mut skin_vertices = skin.map(|_| Vec::with_capacity(indices.len()));

    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = [
            positions[triangle[0]],
            positions[triangle[1]],
            positions[triangle[2]],
        ];
        let normal = (b - a).cross(c - a).normalize_or_zero();
        for &index in triangle {
            vertices.push(Vertex3D {
                pos: positions[index],
                normal,
                uv: uvs.map_or_else(Point::default, |uvs| uvs[index]),
            });
            if let (Some(out), Some(skin)) = (&mut skin_vertices, skin) {
                out.push(skin[index]);
            }
        }
    }

    Geometry {
        indices: (0..vertices.len()).collect(),
        vertices,
        skin: skin_vertices,
    }
}

/// Splits a primitive of `count` vertices into meshes of at most
/// `MAX_VERTICES` vertices, whole triangles only, so every part draws
/// with 16 bit indices. Each part is the source index of its vertices
/// and its own indices.
fn split_u16(count: usize, indices: &[usize]) -> Vec<(Vec<usize>, Vec<u16>)> {
    let index = |i: usize| u16::try_from(i).expect("a part holds at most 65535 vertices");

    if count <= MAX_VERTICES {
        return vec![((0..count).collect(), indices.iter().map(|&i| index(i)).collect())];
    }

    let mut parts = vec![];
    let mut remap: Vec<Option<u16>> = vec![None; count];
    let mut part_sources: Vec<usize> = vec![];
    let mut part_indices: Vec<u16> = vec![];

    for triangle in indices.as_chunks::<3>().0 {
        let new = triangle.iter().filter(|&&i| remap[i].is_none()).count();
        if part_sources.len() + new > MAX_VERTICES {
            parts.push((take(&mut part_sources), take(&mut part_indices)));
            remap.fill(None);
        }
        for &i in triangle {
            let slot = *remap[i].get_or_insert_with(|| {
                part_sources.push(i);
                index(part_sources.len() - 1)
            });
            part_indices.push(slot);
        }
    }

    if !part_indices.is_empty() {
        parts.push((part_sources, part_indices));
    }

    parts
}

fn material_source(material: &gltf::Material) -> Option<MaterialSource> {
    // The default material has no index, the node's material stands in.
    material.index()?;

    let pbr = material.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    // Only a blended material is translucent, a masked one draws solid.
    let alpha = if material.alpha_mode() == AlphaMode::Blend {
        a
    } else {
        1.0
    };
    let normal = material.normal_texture();

    Some(MaterialSource {
        // glTF factors are linear light, the engine's colors are encoded.
        color:        Color::rgba(r, g, b, alpha).encoded(),
        metallic:     pbr.metallic_factor(),
        roughness:    pbr.roughness_factor(),
        texture:      pbr.base_color_texture().map(|info| info.texture().source().index()),
        normal_map:   normal.as_ref().map(|normal| normal.texture().source().index()),
        normal_scale: normal.map_or(1.0, |normal| normal.scale()),
    })
}

/// The node tree with its skins and clips, for a file that has either.
fn rig(document: &Document, blob: &[u8], name: &str) -> Result<Option<Rig>> {
    if document.skins().next().is_none() && document.animations().next().is_none() {
        return Ok(None);
    }

    let count = document.nodes().count();
    let mut parents = vec![None; count];
    for node in document.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }

    let nodes: Vec<RigNode> = document
        .nodes()
        .map(|node| {
            let (translation, rotation, scale) = node.transform().decomposed();
            RigNode {
                parent:      parents[node.index()],
                translation: Vec3::from(translation),
                rotation:    Quat::from_array(rotation),
                scale:       Vec3::from(scale),
            }
        })
        .collect();

    let mut order = Vec::with_capacity(count);
    for root in document.nodes().filter(|node| parents[node.index()].is_none()) {
        push_order(&root, &mut order);
    }

    let skins = document
        .skins()
        .map(|skin| {
            let joints: Vec<usize> = skin.joints().map(|joint| joint.index()).collect();
            let reader = skin.reader(|buffer| (buffer.index() == 0).then_some(blob));
            let inverse_bind: Vec<Mat4> = match reader.read_inverse_bind_matrices() {
                Some(matrices) => matrices.map(|matrix| Mat4::from_cols_array_2d(&matrix)).collect(),
                None => vec![Mat4::IDENTITY; joints.len()],
            };
            ensure!(
                inverse_bind.len() == joints.len(),
                "{name}: skin {} has {} inverse bind matrices for {} joints",
                skin.index(),
                inverse_bind.len(),
                joints.len()
            );
            Ok(Skin { joints, inverse_bind })
        })
        .collect::<Result<Vec<_>>>()?;

    let clips = document
        .animations()
        .map(|animation| clip(&animation, blob, name))
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(Rig {
        nodes,
        order,
        skins,
        clips,
    }))
}

fn push_order(node: &Node, order: &mut Vec<usize>) {
    order.push(node.index());
    for child in node.children() {
        push_order(&child, order);
    }
}

fn clip(animation: &gltf::Animation, blob: &[u8], name: &str) -> Result<Clip> {
    let clip_name = animation.name().unwrap_or_default().to_string();
    let mut channels = vec![];

    for channel in animation.channels() {
        let reader = channel.reader(|buffer| (buffer.index() == 0).then_some(blob));
        let times: Vec<f32> = reader
            .read_inputs()
            .ok_or_else(|| anyhow!("{name}: a channel of {clip_name} has no key times"))?
            .collect();
        ensure!(
            !times.is_empty() && times.is_sorted(),
            "{name}: a channel of {clip_name} has no keys or unsorted keys"
        );

        let interpolation = match channel.sampler().interpolation() {
            KeyInterpolation::Linear => Interpolation::Linear,
            KeyInterpolation::Step => Interpolation::Step,
            KeyInterpolation::CubicSpline => Interpolation::CubicSpline,
        };
        let per_key = if interpolation == Interpolation::CubicSpline {
            3
        } else {
            1
        };

        let outputs = reader
            .read_outputs()
            .ok_or_else(|| anyhow!("{name}: a channel of {clip_name} has no values"))?;
        let track = match outputs {
            ReadOutputs::Translations(values) => Track::Translation(values.map(Vec3::from).collect()),
            ReadOutputs::Scales(values) => Track::Scale(values.map(Vec3::from).collect()),
            ReadOutputs::Rotations(values) => {
                Track::Rotation(values.into_f32().map(Quat::from_array).collect())
            }
            ReadOutputs::MorphTargetWeights(_) => {
                warn!(
                    "{name}: {clip_name} animates morph targets, which do not load, the channel is skipped"
                );
                continue;
            }
        };
        let values = match &track {
            Track::Translation(values) | Track::Scale(values) => values.len(),
            Track::Rotation(values) => values.len(),
        };
        ensure!(
            values == times.len() * per_key,
            "{name}: a channel of {clip_name} has {values} values for {} keys",
            times.len()
        );
        ensure!(
            matches!(
                channel.target().property(),
                Property::Translation | Property::Rotation | Property::Scale
            ),
            "{name}: a channel of {clip_name} targets an unknown property"
        );

        channels.push(Channel {
            node: channel.target().node().index(),
            times,
            interpolation,
            track,
        });
    }

    let duration = channels
        .iter()
        .filter_map(|channel| channel.times.last().copied())
        .fold(0.0, f32::max);

    Ok(Clip {
        name: clip_name,
        duration,
        channels,
    })
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models").join(name);
        fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
    }

    #[test]
    fn monkey_is_one_part_without_a_material_around_the_origin() {
        let model = parse_glb(&fixture("Monkey.glb"), "Monkey.glb").unwrap();
        assert_eq!(model.parts.len(), 1);
        assert!(model.images.is_empty());
        assert!(model.rig.is_none());
        let part = &model.parts[0];
        assert!(part.material.is_none());
        assert!(part.skin.is_none() && part.skin_vertices.is_none());
        assert!(part.vertices.len() > 500);
        assert_eq!(part.indices.len() % 3, 0);
        assert!(part.vertices.iter().all(|vertex| vertex.normal.length() > 0.99));
        // Suzanne is wider than tall, and sits a little off the origin
        // in this file.
        let size = model.bounds.size();
        assert!(size.x > size.y && size.y > size.z, "{:?}", model.bounds);
        assert!(model.bounds.center().length() < 0.2, "{:?}", model.bounds);
    }

    #[test]
    fn tree_carries_its_nodes_and_materials() {
        let model = parse_glb(&fixture("tree.glb"), "tree.glb").unwrap();
        assert!(model.parts.len() >= 5, "{} parts", model.parts.len());
        let materials: Vec<MaterialSource> = model.parts.iter().filter_map(|part| part.material).collect();
        assert_eq!(materials.len(), model.parts.len(), "every mesh has a material");
        // The leaves are green, the trunk brown.
        assert!(materials.iter().any(|material| material.color.g > material.color.r));
        assert!(materials.iter().any(|material| material.color.r > material.color.g));
        // The nodes are placed, not all at the origin.
        assert!(model.parts.iter().any(|part| part.transform.w_axis.truncate().length() > 0.1));
    }

    #[test]
    fn textured_cube_embeds_its_texture() {
        let model = parse_glb(&fixture("textured_cube.glb"), "textured_cube.glb").unwrap();
        assert_eq!(model.images.len(), 1);
        assert_eq!(model.images[0].name, "textured_cube.glb#0");
        assert!(model.images[0].bytes.starts_with(&[0xff, 0xd8]), "jpeg bytes");
        let material = model.parts[0].material.expect("the cube has a material");
        assert_eq!(material.texture, Some(0));
        assert!(model.parts[0].vertices.iter().any(|vertex| vertex.uv != Point::default()));
    }

    // The bar is skinned to a chain of four bones and carries the one
    // second bend of the chain. Its joints place it, so the part has no
    // transform of its own, and at rest the joints move nothing.
    #[test]
    fn bone_test_carries_its_skin_and_the_bend() {
        let model = parse_glb(&fixture("BoneTest.glb"), "BoneTest.glb").unwrap();
        let rig = model.rig.as_ref().expect("a rig");
        assert_eq!(rig.skins.len(), 1);
        assert_eq!(rig.skins[0].joints.len(), 4);
        assert_eq!(rig.skins[0].inverse_bind.len(), 4);
        assert_eq!(model.rest_joints.len(), 1);
        assert!(
            model.rest_joints[0].iter().all(|joint| joint.abs_diff_eq(Mat4::IDENTITY, 1e-4)),
            "{:?}",
            model.rest_joints[0]
        );

        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.skin, Some(0));
        assert_eq!(part.transform, Mat4::IDENTITY);
        let skin = part.skin_vertices.as_ref().expect("skin vertices");
        assert_eq!(skin.len(), part.vertices.len());
        assert!(
            skin.iter()
                .all(|vertex| (vertex.weights.iter().sum::<f32>() - 1.0).abs() < 1e-3)
        );
        assert!(skin.iter().any(|vertex| vertex.joints.iter().any(|&joint| joint > 0)));

        assert_eq!(rig.clips.len(), 1);
        let clip = &rig.clips[0];
        assert_eq!(clip.name, "Bend");
        assert!((clip.duration - 25.0 / 24.0).abs() < 1e-3, "{}", clip.duration);
        assert!(clip.channels.iter().any(|channel| matches!(channel.track, Track::Rotation(_))));

        // Half way through the bend the tip joint has risen, the bar bends
        // upwards, not sideways.
        let bent = rig.pose(Some((clip, clip.duration / 2.0)));
        let rest = rig.pose(None);
        let tip = rig.skins[0].joints[3];
        let lift = bent[tip].w_axis.y - rest[tip].w_axis.y;
        let sideways = (bent[tip].w_axis.z - rest[tip].w_axis.z).abs();
        assert!(
            lift > 3.0 && lift > sideways * 4.0,
            "lift {lift}, sideways {sideways}"
        );
        // The bar is long along x and at rest its bounds are the mesh.
        let size = model.bounds.size();
        assert!(
            size.x > 15.0 && size.y < 3.0 && size.z < 3.0,
            "{:?}",
            model.bounds
        );
    }

    // The Khronos sample fox: one skin of 24 joints, three clips, a
    // texture and no normals, so it shades flat and the skin follows the
    // split vertices.
    #[test]
    fn fox_carries_its_skin_texture_and_three_clips() {
        let model = parse_glb(&fixture("Fox.glb"), "Fox.glb").unwrap();
        let rig = model.rig.as_ref().expect("a rig");
        assert_eq!(rig.skins.len(), 1);
        assert_eq!(rig.skins[0].joints.len(), 24);
        let names: Vec<&str> = rig.clips.iter().map(|clip| clip.name.as_str()).collect();
        assert_eq!(names, ["Survey", "Walk", "Run"]);
        assert!(rig.clips.iter().all(|clip| clip.duration > 0.5));
        assert_eq!(model.images.len(), 1);
        assert!(model.parts.iter().all(|part| part.skin == Some(0)));
        for part in &model.parts {
            assert_eq!(part.material.expect("the fox material").texture, Some(0));
            let skin = part.skin_vertices.as_ref().expect("skin vertices");
            assert_eq!(skin.len(), part.vertices.len());
            assert_eq!(part.indices.len(), part.vertices.len());
        }
        // Standing on its feet, longer than tall, taller than wide.
        let size = model.bounds.size();
        assert!(
            model.bounds.min.y > -1.0 && size.z > size.y && size.y > size.x,
            "{:?}",
            model.bounds
        );
    }

    // The windmill has no skin, its blades are plain parts on the hub
    // node the clip turns, and the post stays put.
    #[test]
    fn windmill_blades_ride_on_the_spinning_hub() {
        let model = parse_glb(&fixture("windmill.glb"), "windmill.glb").unwrap();
        let rig = model.rig.as_ref().expect("a rig");
        assert!(rig.skins.is_empty());
        assert!(
            model
                .parts
                .iter()
                .all(|part| part.skin.is_none() && part.skin_vertices.is_none())
        );
        assert_eq!(rig.clips.len(), 1);
        let clip = &rig.clips[0];
        assert_eq!(clip.name, "Spin");
        assert!((clip.duration - 49.0 / 24.0).abs() < 1e-3, "{}", clip.duration);
        assert_eq!(clip.channels.len(), 1);
        let hub = clip.channels[0].node;

        let blades: Vec<&PartSource> = model
            .parts
            .iter()
            .filter(|part| rig.nodes[part.node].parent == Some(hub))
            .collect();
        assert_eq!(blades.len(), 4);
        let post = model
            .parts
            .iter()
            .find(|part| part.node != hub && rig.nodes[part.node].parent != Some(hub))
            .expect("the post");

        let rest = rig.pose(None);
        let turned = rig.pose(Some((clip, clip.duration / 4.0)));
        assert_eq!(turned[post.node], rest[post.node]);
        for blade in blades {
            assert_eq!(blade.transform, rest[blade.node]);
            let tip = Vec3::Y;
            let moved = turned[blade.node]
                .transform_point3(tip)
                .distance(rest[blade.node].transform_point3(tip));
            assert!(moved > 0.5, "a blade tip moved {moved}");
        }
    }

    #[test]
    fn every_shipped_model_parses() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models");
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "glb") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let model = parse_glb(&fs::read(&path).unwrap(), &name).unwrap();
                assert!(!model.parts.is_empty(), "{name} has no parts");
                assert!(
                    model.parts.iter().all(|part| part.vertices.len() <= MAX_VERTICES),
                    "{name} has a part over the 16 bit limit"
                );
            }
        }
    }

    #[test]
    fn missing_normals_shade_flat_and_keep_the_skin() {
        let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let skin = [
            SkinVertex {
                joints:  [0, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
            SkinVertex {
                joints:  [1, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
            SkinVertex {
                joints:  [2, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
        ];
        let geometry = flat_shaded(&positions, None, Some(&skin), &[0, 2, 1]);
        assert_eq!(geometry.vertices.len(), 3);
        assert_eq!(geometry.indices, vec![0, 1, 2]);
        assert!(geometry.vertices.iter().all(|vertex| vertex.normal == Vec3::NEG_Z));
        let skinned = geometry.skin.expect("the skin follows the vertices");
        assert_eq!(skinned[1].joints[0], 2);
        assert_eq!(skinned[2].joints[0], 1);
    }

    #[test]
    fn a_big_primitive_splits_into_parts_of_whole_triangles() {
        let count = MAX_VERTICES + 10;
        // A fan, every triangle shares vertex zero.
        let indices: Vec<usize> = (1..count - 1).flat_map(|i| [0, i, i + 1]).collect();
        let parts = split_u16(count, &indices);
        assert_eq!(parts.len(), 2);
        let total: usize = parts.iter().map(|(_, indices)| indices.len()).sum();
        assert_eq!(total, indices.len());
        for (sources, indices) in &parts {
            assert!(sources.len() <= MAX_VERTICES);
            assert_eq!(indices.len() % 3, 0);
            assert!(indices.iter().all(|&i| usize::from(i) < sources.len()));
        }
        // Vertex zero is in both parts, once each.
        assert!(
            parts
                .iter()
                .all(|(sources, _)| sources.iter().filter(|&&i| i == 0).count() == 1)
        );
        assert!(parts[1].0.contains(&(count - 1)));
    }

    #[test]
    fn a_small_primitive_is_one_part() {
        let parts = split_u16(3, &[0, 1, 2]);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].0, vec![0, 1, 2]);
        assert_eq!(parts[0].1, vec![0, 1, 2]);
    }
}
