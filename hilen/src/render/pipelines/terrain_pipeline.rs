use bytemuck::{Pod, Zeroable, cast_slice};
use rustc_hash::FxHashMap;
use wgpu::{
    BindGroupLayout, Buffer, BufferUsages, CompareFunction, IndexFormat, PipelineLayoutDescriptor,
    PrimitiveTopology, RenderPass, RenderPipeline, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

use crate::{
    deps::refs::{Own, Weak},
    gm::{
        LossyConvert, checked_usize_to_u32,
        color::WHITE,
        flat::{Point, Rect, Vertex2D},
    },
    level::{MaterialId, Terrain, terrain::mesh::PartKind},
    render::{
        PipelineShape,
        device_helper::DeviceHelper,
        shader_data::SpriteView,
        uniform::{UniformBind, make_uniform_layout},
        vertex_layout::VertexLayout,
    },
    window::{Window, image::Image, msaa_sample_count},
};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
struct TerrainDraw {
    uv_scale: Point,
    z:        f32,
    /// 0 fill, 1 surface strip, 2 edge line, 3 blend band, 4 surface end.
    kind:     u32,
    color:    [f32; 4],
}

#[derive(Debug)]
struct GpuPart {
    material: MaterialId,
    kind:     PartKind,
    vertices: Buffer,
    indices:  Buffer,
    count:    u32,
}

#[derive(Debug)]
struct GpuChunk {
    version: u64,
    parts:   Vec<GpuPart>,
}

/// The parts of every visible chunk draw kind by kind in this order, so a
/// grass strip hanging into the next chunk stays over that chunk's fill.
/// Depth steps between the kinds would be a few float steps apart at the
/// terrain depth and fight, a pixel wide end piece flickered in and out.
const PAINT_ORDER: [PartKind; 5] = [
    PartKind::Fill,
    PartKind::Blend,
    PartKind::Edge,
    PartKind::Surface,
    PartKind::SurfaceEnd,
];

/// Which terrain, by the stamp of its `Own`, and which chunk of it.
type ChunkId = (u64, (i32, i32));

/// Draws the ground of every terrain. The triangles of a chunk stay on
/// the GPU until a carve changes that chunk, only the chunks on screen
/// are drawn.
#[derive(Debug)]
pub struct TerrainPipeline {
    pipeline:    RenderPipeline,
    view:        UniformBind<SpriteView>,
    draw_layout: BindGroupLayout,
    chunks:      FxHashMap<ChunkId, GpuChunk>,
    draws:       FxHashMap<(u64, MaterialId, PartKind), UniformBind<TerrainDraw>>,
}

impl TerrainPipeline {
    /// Built for a pass with `samples` samples a pixel.
    pub(crate) fn with_samples(samples: u32) -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("terrain.wgsl"),
            source: ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/sprite_view.wgsl"),
                    include_str!("shaders/terrain.wgsl")
                )
                .into(),
            ),
        });

        let view_layout = make_uniform_layout("terrain_sprite_view_layout", ShaderStages::VERTEX_FRAGMENT);
        let draw_layout = make_uniform_layout("terrain_draw_layout", ShaderStages::VERTEX_FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "terrain_pipeline_layout".into(),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(&draw_layout),
                Some(Image::uniform_layout()),
            ],
            immediate_size:     0,
        });

        let pipeline = device.pipeline_with(
            "terrain_pipeline",
            &layout,
            &shader,
            &[Vertex2D::VERTEX_LAYOUT],
            PipelineShape {
                // Every part of a terrain lies at one depth and draws in
                // `PAINT_ORDER`, so a later part wins over an earlier one.
                depth_compare: CompareFunction::LessEqual,
                topology: PrimitiveTopology::TriangleList,
                samples,
            },
        );

        Self {
            pipeline,
            view: view_layout.into(),
            draw_layout,
            chunks: FxHashMap::default(),
            draws: FxHashMap::default(),
        }
    }
}

fn uv_scale(image: Weak<Image>, width: f32, strip: bool) -> Point {
    let size = image.size;
    if size.width == 0 || size.height == 0 || width <= 0.0 {
        return Point::default();
    }
    let (w, h): (f32, f32) = (size.width.lossy_convert(), size.height.lossy_convert());
    let aspect = h / w;
    if strip {
        Point::new(1.0 / (width / aspect), 0.0)
    } else {
        Point::new(1.0 / width, 1.0 / (width * aspect))
    }
}

impl TerrainPipeline {
    /// Uploads the chunks a carve changed since the last frame, draws every
    /// chunk that touches `visible`, and frees the chunks of terrains and
    /// ground that are gone.
    pub(crate) fn draw(
        &mut self,
        pass: &mut RenderPass,
        view: &SpriteView,
        terrains: &[Own<Terrain>],
        visible: Rect,
    ) {
        self.chunks.retain(|(stamp, key), _| {
            terrains
                .iter()
                .any(|terrain| terrain.raw().stamp() == *stamp && terrain.chunk_version(*key).is_some())
        });
        if terrains.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        self.view.update(*view);
        pass.set_bind_group(0, self.view.bind(), &[]);

        for terrain in terrains {
            let stamp = terrain.raw().stamp();
            // A strip hangs below its chunk, so a chunk just above the
            // screen can still reach into it.
            let reach = terrain.surface_reach();
            let rect = Rect::new(
                visible.origin.x,
                visible.origin.y,
                visible.size.width,
                visible.size.height + reach,
            );

            let mut keys = vec![];
            for (key, version) in terrain.visible_chunks(rect) {
                let chunk = self.chunks.entry((stamp, key)).or_insert_with(|| GpuChunk {
                    version: u64::MAX,
                    parts:   vec![],
                });
                if chunk.version != version {
                    *chunk = upload(terrain, key, version);
                }
                keys.push(key);
            }

            for kind in PAINT_ORDER {
                for key in &keys {
                    let Some(chunk) = self.chunks.get(&(stamp, *key)) else {
                        continue;
                    };
                    for part in chunk.parts.iter().filter(|part| part.kind == kind) {
                        let material = terrain.material(part.material);
                        // An edge line draws no image, but the pipeline
                        // still binds one, the fill.
                        let (image, width, color, code) = match part.kind {
                            PartKind::Fill => (material.fill, material.fill_size, WHITE, 0),
                            PartKind::Surface => {
                                let Some(surface) = &material.surface else {
                                    continue;
                                };
                                (surface.image, surface.height, WHITE, 1)
                            }
                            PartKind::Edge => {
                                let Some(edge) = &material.edge else {
                                    continue;
                                };
                                (material.fill, material.fill_size, edge.color, 2)
                            }
                            PartKind::Blend => (material.fill, material.fill_size, WHITE, 3),
                            PartKind::SurfaceEnd => {
                                let Some(end) =
                                    material.surface.as_ref().and_then(|surface| surface.end.as_ref())
                                else {
                                    continue;
                                };
                                (end.image, end.width, WHITE, 4)
                            }
                        };
                        if !image.is_ok() {
                            continue;
                        }

                        let data = TerrainDraw {
                            uv_scale: uv_scale(image, width, part.kind == PartKind::Surface),
                            z:        terrain.z_position,
                            kind:     code,
                            color:    [color.r, color.g, color.b, color.a],
                        };
                        let draw = self
                            .draws
                            .entry((stamp, part.material, part.kind))
                            .or_insert_with(|| self.draw_layout.clone().into());
                        draw.update(data);

                        pass.set_bind_group(1, draw.bind(), &[]);
                        pass.set_bind_group(2, image.bind(), &[]);
                        pass.set_vertex_buffer(0, part.vertices.slice(..));
                        pass.set_index_buffer(part.indices.slice(..), IndexFormat::Uint32);
                        pass.draw_indexed(0..part.count, 0, 0..1);
                    }
                }
            }
        }
    }
}

fn upload(terrain: &Terrain, key: (i32, i32), version: u64) -> GpuChunk {
    let device = Window::device();
    let parts = terrain
        .chunk_mesh(key)
        .into_iter()
        .map(|part| GpuPart {
            material: part.material,
            kind:     part.kind,
            vertices: device.buffer_from_bytes(cast_slice(&part.vertices), BufferUsages::VERTEX),
            indices:  device.buffer_from_bytes(cast_slice(&part.indices), BufferUsages::INDEX),
            count:    checked_usize_to_u32(part.indices.len()),
        })
        .collect();
    GpuChunk { version, parts }
}

impl Default for TerrainPipeline {
    fn default() -> Self {
        Self::with_samples(msaa_sample_count())
    }
}
