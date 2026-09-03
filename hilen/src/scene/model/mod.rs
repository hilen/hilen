mod parse;
mod rig;

use std::path::Path;

use log::error;

use self::parse::{ModelSource, parse_glb};
pub use self::rig::Clip;
pub(crate) use self::rig::Rig;
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
/// Meshes, materials, embedded textures, the node tree, skins and
/// animation clips load. A static model has its tree flattened into
/// parts, one with a skin or a clip keeps the tree as its `rig` and a
/// node poses it, see `NodeTemplates::play`.
#[derive(Debug)]
pub struct Model {
    pub(crate) parts:       Vec<ModelPart>,
    /// The box around every vertex in model space, at rest for a
    /// skinned model. The collider of a model node is this box.
    pub bounds:             Bounds,
    pub(crate) rig:         Option<Rig>,
    /// Every skin's joint matrices at rest, what a node draws with
    /// while no clip plays.
    pub(crate) rest_joints: Vec<Vec<Mat4>>,
}

/// One drawn mesh of a model with its place and look.
#[derive(Debug)]
pub(crate) struct ModelPart {
    pub mesh:      Own<Mesh>,
    /// Model space to the part at rest, every parent node applied.
    /// Identity for a skinned part, its joints place it.
    pub transform: Mat4,
    /// The node of the rig this part hangs on, what a clip moves.
    pub node:      usize,
    /// The skin over the part, an index into the rig's skins.
    pub skin:      Option<usize>,
    /// None when the glTF primitive has no material, then the node's
    /// own material draws it.
    pub material:  Option<Material>,
}

managed!(Model);

impl Model {
    /// The animation clips of the file, in its order.
    pub fn clips(&self) -> &[Clip] {
        self.rig.as_ref().map_or(&[], |rig| &rig.clips)
    }

    /// The clip called `name`, as an index into `clips`.
    pub fn clip(&self, name: &str) -> Option<usize> {
        self.clips().iter().position(|clip| clip.name == name)
    }

    fn empty() -> Self {
        Self {
            parts:       vec![],
            bounds:      Bounds::default(),
            rig:         None,
            rest_joints: vec![],
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
                mesh:      Own::new(match &part.skin_vertices {
                    Some(skin) => Mesh::upload_skinned(&part.vertices, skin, &part.indices),
                    None => Mesh::upload(&part.vertices, &part.indices),
                }),
                transform: part.transform,
                node:      part.node,
                skin:      part.skin,
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
            rig: source.rig,
            rest_joints: source.rest_joints,
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
