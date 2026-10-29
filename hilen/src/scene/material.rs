use crate::{deps::refs::Weak, gm::color::Color, window::image::Image};

/// How the surface of a node meets light, the metallic roughness model
/// of glTF and Filament.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Encoded sRGB like every color. The diffuse color of a dielectric,
    /// the tint of the reflection of a metal. An alpha below one makes
    /// the node translucent, drawn after every opaque node, back to
    /// front, unless `cutout` is set.
    pub color:                  Color,
    /// 0 is a dielectric, plastic, stone or paint. 1 is a metal, which
    /// has no diffuse and colors its reflection.
    pub metallic:               f32,
    /// 0 is a mirror, 1 is fully matte.
    pub roughness:              f32,
    /// Multiplies `color` per texel, encoded sRGB like every image, its
    /// alpha multiplies the alpha.
    pub texture:                Option<Weak<Image>>,
    /// The metallic in blue and the roughness in green, each multiplies
    /// its factor per texel, the glTF packing. Read as plain data, not
    /// as a color.
    pub metallic_roughness_map: Option<Weak<Image>>,
    /// A tangent space normal map, the glTF convention with green up.
    /// No tangents are needed, the shader builds the frame from the
    /// derivatives of the position and the uv.
    pub normal_map:             Option<Weak<Image>>,
    /// How far the normal map tilts the normal, 1 as painted, more for
    /// deeper relief, the glTF normal scale.
    pub normal_scale:           f32,
    /// How much light from all around reaches each texel, in red, 0 for
    /// none and 1 for all. It darkens the light of the sky or of the
    /// flat ambient, not the sun or a lamp, as glTF asks.
    pub occlusion_map:          Option<Weak<Image>>,
    /// How far the occlusion map darkens, 0 not at all and 1 as painted,
    /// the glTF occlusion strength.
    pub occlusion_strength:     f32,
    /// Light the surface gives off itself, lit or in the dark: its color
    /// times this is added to what the lights give it. 0 for none, 1
    /// shows the full color even at night, more blooms past it. Flames
    /// and lamps glow this way. It lights nothing around it, a point
    /// light does that.
    pub emissive:               f32,
    /// Draws the color times the texture as it is, with no light and no
    /// shadow on it, what `KHR_materials_unlit` of glTF asks. A photo
    /// scan carries its light in its texture already. The fog still
    /// covers it.
    pub unlit:                  bool,
    /// Reflects like the dust of a moon, by the Lommel-Seeliger law in
    /// place of Lambert's: brightness goes with `2 cos i / (cos i +
    /// cos e)`, `i` the angle to the light and `e` to the eye. Lit from
    /// behind the eye a dusty ball is flat and bright to its edge, a full
    /// moon, where a Lambert ball darkens. Facing the light and the eye
    /// both laws give the same brightness, so dust fits next to the rest.
    pub dust:                   bool,
    /// Cuts the shape of the alpha out of the node, leaves on a quad: a
    /// fragment whose alpha, the color's times the texel's, is under
    /// this is not drawn at all, and the rest draws solid, with depth,
    /// lit and casting the shadow of what is left. `None` draws every
    /// fragment. `Material::CUTOUT` is the usual threshold.
    pub cutout:                 Option<f32>,
    /// How much of the node draws, 1 all of it and 0 none. Below 1 the
    /// node leaves out its pixels in a 4 by 4 ordered dither pattern and
    /// the rest draws solid, with depth, casting the shadow of what is
    /// left, so a level of detail or a new tree fades in without the sort
    /// a translucent node needs. `NodeTemplates::fade_to` runs it over
    /// time.
    pub fade:                   f32,
    /// Draws the other share of the dither pattern. A node fading in and a
    /// flipped one fading out at the same place, with fades that add up to
    /// 1, cover every pixel once, a cross fade with no hole.
    pub fade_flip:              bool,
}

impl Material {
    /// The threshold glTF gives a masked material that names none.
    pub const CUTOUT: f32 = 0.5;
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color:                  Color::random(),
            metallic:               0.0,
            roughness:              0.5,
            texture:                None,
            metallic_roughness_map: None,
            normal_map:             None,
            normal_scale:           1.0,
            occlusion_map:          None,
            occlusion_strength:     1.0,
            emissive:               0.0,
            unlit:                  false,
            dust:                   false,
            cutout:                 None,
            fade:                   1.0,
            fade_flip:              false,
        }
    }
}
