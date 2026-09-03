mod parse;

use std::path::Path;

use log::error;

use self::parse::{ModelSource, parse_glb};
use crate::{
    deps::{
        hreads::from_main,
        refs::{
            Own,
            manage::{DataManager, ResourceLoader},
        },
    },
    filesystem::read_bytes,
    gm::volume::{Bounds, Mat4},
    managed,
    scene::{Material, Mesh},
    window::image::Image,
};

/// A `.glb` loaded onto the GPU, a managed resource like an `Image`.
/// `Model::get("tree.glb")` reads it from the models asset folder once
/// and every node with `Shape3::Model` of it draws the same buffers.
/// Meshes, materials, embedded textures and the node tree load, the
/// tree flattened into parts. Static geometry only, no skins or
/// animations yet.
#[derive(Debug)]
pub struct Model {
    pub(crate) parts: Vec<ModelPart>,
    /// The box around every vertex in model space. The collider of a
    /// model node is this box.
    pub bounds:       Bounds,
}

/// One drawn mesh of a model with its place and look.
#[derive(Debug)]
pub(crate) struct ModelPart {
    pub mesh:      Own<Mesh>,
    /// Model space to the part, every parent node applied.
    pub transform: Mat4,
    /// None when the glTF primitive has no material, then the node's
    /// own material draws it.
    pub material:  Option<Material>,
}

managed!(Model);

impl Model {
    fn empty() -> Self {
        Self {
            parts:  vec![],
            bounds: Bounds::default(),
        }
    }

    fn upload(source: ModelSource) -> Self {
        let images: Vec<_> = source
            .images
            .iter()
            .map(|image| Image::load(&image.bytes, &image.name))
            .collect();

        let parts = source
            .parts
            .into_iter()
            .map(|part| ModelPart {
                mesh:      Own::new(Mesh::upload(&part.vertices, &part.indices)),
                transform: part.transform,
                material:  part.material.map(|material| Material {
                    color:        material.color,
                    metallic:     material.metallic,
                    roughness:    material.roughness,
                    texture:      material.texture.map(|index| images[index]),
                    normal_map:   material.normal_map.map(|index| images[index]),
                    normal_scale: material.normal_scale,
                }),
            })
            .collect();

        Self {
            parts,
            bounds: source.bounds,
        }
    }
}

impl ResourceLoader for Model {
    fn load_path(path: &Path) -> Self {
        match read_bytes(path) {
            Ok(data) => Self::load_data(&data, path.display()),
            Err(err) => {
                error!(
                    "Failed to read model file: {}. Error: {err} Returning an empty model",
                    path.display()
                );
                Self::empty()
            }
        }
    }

    fn load_data(data: &[u8], name: impl ToString) -> Self {
        let name = name.to_string();

        let source =
            parse_glb(data, &name).unwrap_or_else(|err| panic!("Failed to load model {name}. Err: {err}"));

        from_main(move || Self::upload(source))
    }
}
