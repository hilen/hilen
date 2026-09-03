use std::mem::take;

use anyhow::{Result, anyhow, bail, ensure};
use gltf::{Gltf, Node, Primitive, buffer, image, material::AlphaMode, mesh::Mode};

#[cfg(test)]
use crate::gm::LossyConvert;
use crate::gm::{
    color::Color,
    flat::Point,
    volume::{Bounds, Mat4, Vec3, Vertex3D},
};

/// Indices are 16 bit, so a mesh holds at most this many vertices and a
/// bigger primitive is split.
const MAX_VERTICES: usize = u16::MAX as usize;

/// Everything a `.glb` holds, decoded on the CPU and ready to upload.
pub(crate) struct ModelSource {
    pub parts:  Vec<PartSource>,
    pub images: Vec<EmbeddedImage>,
    pub bounds: Bounds,
}

/// One primitive of one node, or a slice of a primitive too big for 16
/// bit indices.
pub(crate) struct PartSource {
    pub vertices:  Vec<Vertex3D>,
    pub indices:   Vec<u16>,
    /// The node's place in the model, every parent applied.
    pub transform: Mat4,
    /// None when the primitive has no material, the node's own applies.
    pub material:  Option<MaterialSource>,
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

    let mut parts = vec![];
    for node in scene.nodes() {
        walk(&node, Mat4::IDENTITY, blob, name, &mut parts)?;
    }

    let bounds = Bounds::of_points(
        parts
            .iter()
            .flat_map(|part| part.vertices.iter().map(|vertex| part.transform.transform_point3(vertex.pos))),
    );

    Ok(ModelSource {
        parts,
        images,
        bounds,
    })
}

fn walk(node: &Node, parent: Mat4, blob: &[u8], name: &str, parts: &mut Vec<PartSource>) -> Result<()> {
    let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            parts.extend(primitive_parts(&primitive, transform, blob, name)?);
        }
    }

    for child in node.children() {
        walk(&child, transform, blob, name, parts)?;
    }

    Ok(())
}

fn primitive_parts(
    primitive: &Primitive,
    transform: Mat4,
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

    // A mirroring transform turns the winding inside out, and back faces
    // are culled.
    if transform.determinant() < 0.0 {
        for triangle in indices.as_chunks_mut::<3>().0 {
            triangle.swap(1, 2);
        }
    }

    let vertices: Vec<Vertex3D> = match normals {
        Some(normals) => positions
            .iter()
            .zip(&normals)
            .enumerate()
            .map(|(i, (pos, normal))| Vertex3D {
                pos:    *pos,
                normal: *normal,
                uv:     uvs.as_ref().map_or_else(Point::default, |uvs| uvs[i]),
            })
            .collect(),
        None => flat_shaded(&positions, uvs.as_deref(), &mut indices),
    };

    let material = material_source(&primitive.material());

    Ok(split_u16(&vertices, &indices)
        .into_iter()
        .map(|(vertices, indices)| PartSource {
            vertices,
            indices,
            transform,
            material,
        })
        .collect())
}

/// Without normals glTF asks for flat shading, so every triangle gets
/// its own three vertices carrying the face normal.
fn flat_shaded(positions: &[Vec3], uvs: Option<&[Point]>, indices: &mut Vec<usize>) -> Vec<Vertex3D> {
    let mut vertices = Vec::with_capacity(indices.len());

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
        }
    }

    *indices = (0..vertices.len()).collect();
    vertices
}

/// Splits a primitive into meshes of at most `MAX_VERTICES` vertices,
/// whole triangles only, so every part draws with 16 bit indices.
fn split_u16(vertices: &[Vertex3D], indices: &[usize]) -> Vec<(Vec<Vertex3D>, Vec<u16>)> {
    let index = |i: usize| u16::try_from(i).expect("a part holds at most 65535 vertices");

    if vertices.len() <= MAX_VERTICES {
        return vec![(vertices.to_vec(), indices.iter().map(|&i| index(i)).collect())];
    }

    let mut parts = vec![];
    let mut remap: Vec<Option<u16>> = vec![None; vertices.len()];
    let mut part_vertices: Vec<Vertex3D> = vec![];
    let mut part_indices: Vec<u16> = vec![];

    for triangle in indices.as_chunks::<3>().0 {
        let new = triangle.iter().filter(|&&i| remap[i].is_none()).count();
        if part_vertices.len() + new > MAX_VERTICES {
            parts.push((take(&mut part_vertices), take(&mut part_indices)));
            remap.fill(None);
        }
        for &i in triangle {
            let slot = *remap[i].get_or_insert_with(|| {
                part_vertices.push(vertices[i]);
                index(part_vertices.len() - 1)
            });
            part_indices.push(slot);
        }
    }

    if !part_indices.is_empty() {
        parts.push((part_vertices, part_indices));
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

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models").join(name);
        fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
    }

    fn vertex(x: f32, y: f32, z: f32) -> Vertex3D {
        Vertex3D {
            pos: Vec3::new(x, y, z),
            ..Vertex3D::default()
        }
    }

    #[test]
    fn monkey_is_one_part_without_a_material_around_the_origin() {
        let model = parse_glb(&fixture("Monkey.glb"), "Monkey.glb").unwrap();
        assert_eq!(model.parts.len(), 1);
        assert!(model.images.is_empty());
        let part = &model.parts[0];
        assert!(part.material.is_none());
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
        assert!(model.parts.len() >= 4, "{} parts", model.parts.len());
        let materials: Vec<MaterialSource> = model.parts.iter().filter_map(|part| part.material).collect();
        assert_eq!(
            materials.len(),
            model.parts.len() - 1,
            "every mesh but the plane has a material"
        );
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
    fn missing_normals_shade_flat() {
        let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let mut indices = vec![0, 1, 2];
        let vertices = flat_shaded(&positions, None, &mut indices);
        assert_eq!(vertices.len(), 3);
        assert_eq!(indices, vec![0, 1, 2]);
        assert!(vertices.iter().all(|vertex| vertex.normal == Vec3::Z));
    }

    #[test]
    fn a_big_primitive_splits_into_parts_of_whole_triangles() {
        let count = MAX_VERTICES + 10;
        let vertices: Vec<Vertex3D> = (0..count).map(|i| vertex(i.lossy_convert(), 0.0, 0.0)).collect();
        // A fan, every triangle shares vertex zero.
        let indices: Vec<usize> = (1..count - 1).flat_map(|i| [0, i, i + 1]).collect();
        let parts = split_u16(&vertices, &indices);
        assert_eq!(parts.len(), 2);
        let total: usize = parts.iter().map(|(_, indices)| indices.len()).sum();
        assert_eq!(total, indices.len());
        for (vertices, indices) in &parts {
            assert!(vertices.len() <= MAX_VERTICES);
            assert_eq!(indices.len() % 3, 0);
            assert!(indices.iter().all(|&i| usize::from(i) < vertices.len()));
        }
        // Vertex zero is in both parts, once each.
        assert!(
            parts
                .iter()
                .all(|(vertices, _)| vertices.iter().filter(|v| v.pos == Vec3::ZERO).count() == 1)
        );
    }

    #[test]
    fn a_small_primitive_is_one_part() {
        let vertices = [
            vertex(0.0, 0.0, 0.0),
            vertex(1.0, 0.0, 0.0),
            vertex(0.0, 1.0, 0.0),
        ];
        let parts = split_u16(&vertices, &[0, 1, 2]);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].1, vec![0, 1, 2]);
    }
}
