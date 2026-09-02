use crate::{deps::refs::Weak, gm::color::Color, window::image::Image};

/// How the surface of a node meets light, the metallic roughness model
/// of glTF and Filament.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Encoded sRGB like every color. The diffuse color of a dielectric,
    /// the tint of the reflection of a metal. An alpha below one makes
    /// the node translucent, drawn after every opaque node, back to
    /// front.
    pub color:        Color,
    /// 0 is a dielectric, plastic, stone or paint. 1 is a metal, which
    /// has no diffuse and colors its reflection.
    pub metallic:     f32,
    /// 0 is a mirror, 1 is fully matte.
    pub roughness:    f32,
    /// Multiplies `color` per texel, encoded sRGB like every image, its
    /// alpha multiplies the alpha.
    pub texture:      Option<Weak<Image>>,
    /// A tangent space normal map, the glTF convention with green up.
    /// No tangents are needed, the shader builds the frame from the
    /// derivatives of the position and the uv.
    pub normal_map:   Option<Weak<Image>>,
    /// How far the normal map tilts the normal, 1 as painted, more for
    /// deeper relief, the glTF normal scale.
    pub normal_scale: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color:        Color::random(),
            metallic:     0.0,
            roughness:    0.5,
            texture:      None,
            normal_map:   None,
            normal_scale: 1.0,
        }
    }
}
