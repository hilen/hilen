use hilen_pixels::{LinearCube, ROUGH_LEVELS, irradiance, prefilter, sky_gradient};
use wgpu::{
    AddressMode, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Sampler, SamplerDescriptor,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::{
    gm::{
        color::{Color, U8Color},
        volume::{Vec3, Vec4},
    },
    window::Window,
};

/// Face width of the built in gradient sky.
const GRADIENT_SIZE: usize = 128;

/// The sky of a scene, a cube map. The skybox draws it and every
/// surface reflects it: the diffuse through nine spherical harmonics and
/// the specular through one mip per roughness step, both computed on the
/// CPU when the sky is made, see `hilen_pixels::prefilter`.
#[derive(Clone, Debug)]
pub struct Sky {
    pub(crate) view:       TextureView,
    pub(crate) sampler:    Sampler,
    pub(crate) irradiance: [Vec4; 9],
}

impl Sky {
    /// A smooth sky, `zenith` straight up through `horizon` at eye level
    /// to `ground` straight down.
    pub fn gradient(zenith: Color, horizon: Color, ground: Color) -> Self {
        Self::from_cube(&sky_gradient(
            GRADIENT_SIZE,
            linear(zenith),
            linear(horizon),
            linear(ground),
        ))
    }

    /// From six square faces of RGBA bytes, encoded sRGB, in the order
    /// `+x -x +y -y +z -z`, each `size` texels wide, at least 32.
    pub fn from_faces(size: u32, faces: &[Vec<u8>; 6]) -> Self {
        let size = usize::try_from(size).expect("face size fits usize");
        Self::from_cube(&LinearCube::from_bytes(size, faces))
    }

    fn from_cube(cube: &LinearCube) -> Self {
        let rough = prefilter(cube);
        let coefficients = irradiance(cube);

        let size = u32::try_from(cube.size).expect("face size fits u32");
        let device = Window::device();

        // The bytes stay encoded like every texture of the engine, the
        // shader decodes after the sample.
        let texture = device.create_texture(&TextureDescriptor {
            label:           "sky_cube".into(),
            size:            Extent3d {
                width:                 size,
                height:                size,
                depth_or_array_layers: 6,
            },
            mip_level_count: u32::from(ROUGH_LEVELS) + 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Rgba8Unorm,
            usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats:    &[],
        });

        for (level, cube) in std::iter::once(cube).chain(rough.iter()).enumerate() {
            let level = u32::try_from(level).expect("mip level fits u32");
            let width = u32::try_from(cube.size).expect("face size fits u32");
            for (face, texels) in cube.faces.iter().enumerate() {
                Window::queue().write_texture(
                    TexelCopyTextureInfo {
                        texture:   &texture,
                        mip_level: level,
                        origin:    Origin3d {
                            x: 0,
                            y: 0,
                            z: u32::try_from(face).expect("six faces"),
                        },
                        aspect:    TextureAspect::All,
                    },
                    &encoded_bytes(texels),
                    TexelCopyBufferLayout {
                        offset:         0,
                        bytes_per_row:  Some(4 * width),
                        rows_per_image: Some(width),
                    },
                    Extent3d {
                        width,
                        height: width,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let view = texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "sky_sampler".into(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        Self {
            view,
            sampler,
            irradiance: coefficients.map(|c| c.extend(0.0)),
        }
    }

    /// The binding a scene without a sky gets, black all around, so
    /// the shader has a cube to sample and reflects nothing from it.
    pub(crate) fn black() -> Self {
        let size = 1 << ROUGH_LEVELS;
        Self::from_cube(&LinearCube {
            size,
            faces: std::array::from_fn(|_| vec![Vec3::ZERO; size * size]),
        })
    }
}

fn linear(color: Color) -> Vec3 {
    let color = color.linear();
    Vec3::new(color.r, color.g, color.b)
}

fn encoded_bytes(texels: &[Vec3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(texels.len() * 4);
    for texel in texels {
        let color = U8Color::from(Color::rgb(texel.x, texel.y, texel.z).encoded());
        bytes.extend_from_slice(&[color.r, color.g, color.b, 255]);
    }
    bytes
}
