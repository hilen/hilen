//! The material of a glTF primitive as the loader reads it.

use gltf::material::AlphaMode;

use crate::{gm::color::Color, scene::Material};

/// A glTF material, its textures as indices into `ModelSource::images`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialSource {
    pub color:              Color,
    pub metallic:           f32,
    pub roughness:          f32,
    pub texture:            Option<usize>,
    /// Metallic in blue and roughness in green, each multiplies its
    /// factor.
    pub metallic_roughness: Option<usize>,
    pub normal_map:         Option<usize>,
    pub normal_scale:       f32,
    /// Red darkens the light of the sky or the ambient.
    pub occlusion:          Option<usize>,
    pub occlusion_strength: f32,
    /// The alpha cutoff of a masked material.
    pub cutout:             Option<f32>,
    /// `KHR_materials_unlit`, the base color drawn with no light.
    pub unlit:              bool,
}

pub(crate) fn material_source(material: &gltf::Material) -> Option<MaterialSource> {
    // The default material has no index, the node's material stands in.
    material.index()?;

    let pbr = material.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    // A blended material is translucent and a masked one is cut out by
    // its alpha, an opaque one ignores it.
    let mode = material.alpha_mode();
    let alpha = if mode == AlphaMode::Opaque { 1.0 } else { a };
    let cutout = (mode == AlphaMode::Mask).then(|| material.alpha_cutoff().unwrap_or(Material::CUTOUT));
    let normal = material.normal_texture();
    let occlusion = material.occlusion_texture();

    Some(MaterialSource {
        // glTF factors are linear light, the engine's colors are encoded.
        color: Color::rgba(r, g, b, alpha).encoded(),
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        texture: pbr.base_color_texture().map(|info| info.texture().source().index()),
        metallic_roughness: pbr.metallic_roughness_texture().map(|info| info.texture().source().index()),
        normal_map: normal.as_ref().map(|normal| normal.texture().source().index()),
        normal_scale: normal.map_or(1.0, |normal| normal.scale()),
        occlusion: occlusion.as_ref().map(|occlusion| occlusion.texture().source().index()),
        occlusion_strength: occlusion.map_or(1.0, |occlusion| occlusion.strength()),
        cutout,
        unlit: material.unlit(),
    })
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use crate::scene::model::parse::parse_glb;

    // One texture holds the occlusion, the roughness and the metallic,
    // the way glTF packs them, and the factors stay at 1.
    #[test]
    fn metal_rough_reads_its_metallic_roughness_and_occlusion_texture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/metal_rough.glb");
        let data = fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let model = parse_glb(&data, "metal_rough.glb").unwrap();

        assert_eq!(model.images.len(), 1);
        let material = model.parts[0].material.expect("the quad has a material");
        assert_eq!(material.metallic, 1.0);
        assert_eq!(material.roughness, 1.0);
        assert_eq!(material.texture, None);
        assert_eq!(material.metallic_roughness, Some(0));
        assert_eq!(material.occlusion, Some(0));
        assert_eq!(material.occlusion_strength, 1.0);
        assert!(!material.unlit);
    }

    // The fixture of `deps/models/make_unlit_quad.ts`, a red quad whose
    // material is marked `KHR_materials_unlit`.
    #[test]
    fn unlit_quad_reads_as_unlit() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/unlit_quad.glb");
        let data = fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let model = parse_glb(&data, "unlit_quad.glb").unwrap();

        let material = model.parts[0].material.expect("the quad has a material");
        assert!(material.unlit);
        assert_eq!(material.texture, None);
    }
}
