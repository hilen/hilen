//! The 2 tables of a `scene::Atmosphere` on the GPU, built again only when
//! the shape of the air changes, not when it moves. The tables stay on
//! the CPU too, for the ambient light worked out every frame.

use std::time::Instant;

use wgpu::{
    Extent3d, Origin3d, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::{
    gm::volume::Vec3,
    scene::{Atmosphere, AtmosphereTables, Table, half_bits},
    window::Window,
};

pub(crate) struct AirTextures {
    shape:                    Option<Atmosphere>,
    tables:                   Option<AtmosphereTables>,
    pub(crate) transmittance: TextureView,
    pub(crate) multi:         TextureView,
}

impl AirTextures {
    /// Black 1 texel tables, the binding of a scene with no air.
    pub(crate) fn new() -> Self {
        let empty = Table {
            width:  1,
            height: 1,
            texels: vec![Vec3::ZERO],
        };
        Self {
            shape:         None,
            tables:        None,
            transmittance: upload(&empty, "air_transmittance"),
            multi:         upload(&empty, "air_multi"),
        }
    }

    /// Builds the tables of `air` when its shape is not the one built.
    pub(crate) fn update(&mut self, air: Option<&Atmosphere>) {
        let Some(air) = air else {
            return;
        };
        let shape = air.shape();
        if self.shape == Some(shape) {
            return;
        }
        let started = Instant::now();
        let tables = shape.tables();
        self.transmittance = upload(&tables.transmittance, "air_transmittance");
        self.multi = upload(&tables.multi, "air_multi");
        log::info!(
            "air tables of a planet of {} m built in {} ms",
            air.ground_radius,
            started.elapsed().as_millis()
        );
        self.tables = Some(tables);
        self.shape = Some(shape);
    }

    /// The ambient light of `air` at `camera`, relative to its center,
    /// for a sun of 1 toward `to_sun`, once its tables are built.
    pub(crate) fn ambient(&self, air: &Atmosphere, camera: Vec3, to_sun: Vec3) -> Option<[Vec3; 9]> {
        let tables = self.tables.as_ref().filter(|_| self.shape == Some(air.shape()))?;
        Some(air.ambient(tables, camera, to_sun))
    }
}

fn upload(table: &Table, label: &str) -> TextureView {
    let width = u32::try_from(table.width).expect("a table fits u32");
    let height = u32::try_from(table.height).expect("a table fits u32");
    let texture = Window::device().create_texture(&TextureDescriptor {
        label:           label.into(),
        size:            Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count:    1,
        dimension:       TextureDimension::D2,
        format:          TextureFormat::Rgba16Float,
        usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats:    &[],
    });
    let bytes: Vec<u8> = table
        .texels
        .iter()
        .flat_map(|texel| [texel.x, texel.y, texel.z, 1.0])
        .flat_map(|value| half_bits(value).to_le_bytes())
        .collect();
    Window::queue().write_texture(
        TexelCopyTextureInfo {
            texture:   &texture,
            mip_level: 0,
            origin:    Origin3d::ZERO,
            aspect:    TextureAspect::All,
        },
        &bytes,
        TexelCopyBufferLayout {
            offset:         0,
            bytes_per_row:  Some(8 * width),
            rows_per_image: Some(height),
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&TextureViewDescriptor::default())
}
