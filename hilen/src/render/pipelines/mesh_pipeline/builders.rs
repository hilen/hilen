//! Makes the pipelines of `MeshPipeline`.

use wgpu::{
    CompareFunction, PipelineLayout, PipelineLayoutDescriptor, PrimitiveTopology, RenderPipeline,
    ShaderModule, ShaderModuleDescriptor, ShaderSource,
};

use super::{SKINNED_LAYOUTS, STATIC_LAYOUTS};
use crate::{
    gm::volume::Vertex3D,
    render::{
        data::{GrassBlade, LineVertex, WideLine},
        device_helper::{DeviceHelper, MeshDraw, MeshPass, PipelineShape, SCENE_DEPTH_COMPARE},
        vertex_layout::VertexLayout,
    },
};

const DEPTH_FILL: &str = include_str!("../shaders/depth_fill.wgsl");

const LINE_LAYOUTS: &[wgpu::VertexBufferLayout] = &[LineVertex::VERTEX_LAYOUT];
const WIDE_LINE_LAYOUTS: &[wgpu::VertexBufferLayout] = &[WideLine::VERTEX_LAYOUT];
/// The blade mesh and the blades of a grass patch.
const GRASS_LAYOUTS: &[wgpu::VertexBufferLayout] = &[Vertex3D::VERTEX_LAYOUT, GrassBlade::VERTEX_LAYOUT];

/// The pipelines of a mesh whose colors fade, see
/// `Model::from_smooth_mesh`, the fragment entries that take `v_smooth`.
/// Made at the first such mesh, so an app with none does not wait for them.
/// There is no skinned twin, only a mesh built in code fades.
pub(super) struct SmoothPipelines {
    pub opaque:      RenderPipeline,
    pub cutout:      RenderPipeline,
    pub translucent: RenderPipeline,
}

impl SmoothPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        samples: u32,
    ) -> Self {
        let pipeline =
            |label: &str, draw: MeshDraw| smooth_pipeline(device, label, layout, shader, draw, samples);
        Self {
            opaque:      pipeline("mesh_smooth_pipeline", MeshDraw::SmoothOpaque),
            cutout:      pipeline("mesh_smooth_cutout_pipeline", MeshDraw::SmoothCutout),
            translucent: pipeline("mesh_smooth_translucent_pipeline", MeshDraw::SmoothTranslucent),
        }
    }
}

/// The pipelines of a mesh whose texture slides along a flow, see
/// `Model::from_flowing_mesh`, made at the first such mesh. A plain shadow
/// and a plain depth photo read only the position, they draw it the plain way.
pub(super) struct FlowPipelines {
    pub opaque:      RenderPipeline,
    pub translucent: RenderPipeline,
}

impl FlowPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        samples: u32,
    ) -> Self {
        let pipeline = |label: &str, draw: MeshDraw| {
            device.mesh_pipeline(
                label,
                layout,
                shader,
                STATIC_LAYOUTS,
                "v_flow",
                MeshPass { draw, samples },
            )
        };
        Self {
            opaque:      pipeline("mesh_flow_pipeline", MeshDraw::FlowOpaque),
            translucent: pipeline("mesh_flow_translucent_pipeline", MeshDraw::FlowTranslucent),
        }
    }
}

/// A pipeline of a mesh whose colors fade, `v_smooth` into the fragment
/// entry of `draw`.
pub(super) fn smooth_pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    draw: MeshDraw,
    samples: u32,
) -> RenderPipeline {
    device.mesh_pipeline(
        label,
        layout,
        shader,
        STATIC_LAYOUTS,
        "v_smooth",
        MeshPass { draw, samples },
    )
}

/// The thin lines, then the wide lines that are depth tested and the wide
/// lines on top of everything.
pub(super) fn line_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    lines: &wgpu::ShaderModule,
    wide: &wgpu::ShaderModule,
    samples: u32,
) -> [RenderPipeline; 3] {
    let shape = |depth_compare, topology| PipelineShape {
        depth_compare,
        topology,
        samples,
    };
    [
        device.pipeline_with(
            "lines_pipeline",
            layout,
            lines,
            LINE_LAYOUTS,
            shape(SCENE_DEPTH_COMPARE, PrimitiveTopology::LineList),
        ),
        device.pipeline_with(
            "wide_lines_tested",
            layout,
            wide,
            WIDE_LINE_LAYOUTS,
            shape(SCENE_DEPTH_COMPARE, PrimitiveTopology::TriangleList),
        ),
        device.pipeline_with(
            "wide_lines_on_top",
            layout,
            wide,
            WIDE_LINE_LAYOUTS,
            shape(CompareFunction::Always, PrimitiveTopology::TriangleList),
        ),
    ]
}

/// A mesh pipeline and its skinned twin, the same shader from its two
/// vertex entries.
pub(super) fn mesh_pipelines(
    device: &wgpu::Device,
    name: &str,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    draw: MeshDraw,
    samples: u32,
) -> [RenderPipeline; 2] {
    [
        device.mesh_pipeline(
            &format!("{name}_pipeline"),
            layout,
            shader,
            STATIC_LAYOUTS,
            "v_main",
            MeshPass { draw, samples },
        ),
        device.mesh_pipeline(
            &format!("{name}_skinned_pipeline"),
            layout,
            shader,
            SKINNED_LAYOUTS,
            "v_skinned",
            MeshPass { draw, samples },
        ),
    ]
}

/// The grass of the mesh shader from `v_grass`, the depth of its blades
/// and then their light.
pub(super) fn grass_pipelines(
    device: &wgpu::Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    samples: u32,
) -> [RenderPipeline; 2] {
    [
        ("grass_depth_pipeline", MeshDraw::GrassDepth),
        ("grass_pipeline", MeshDraw::Grass),
    ]
    .map(|(label, draw)| {
        device.mesh_pipeline(
            label,
            layout,
            shader,
            GRASS_LAYOUTS,
            "v_grass",
            MeshPass { draw, samples },
        )
    })
}

/// The 2 passes that write the far end of the scene's depth before it
/// draws and the near end after, see `fill_depth`.
pub(super) fn depth_fill_pipelines(device: &wgpu::Device, samples: u32) -> [RenderPipeline; 2] {
    let fill_shader = shader(device, "depth_fill_shader", DEPTH_FILL.to_string());
    let fill_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label:              "depth_fill_layout".into(),
        bind_group_layouts: &[],
        immediate_size:     0,
    });
    [("depth_far", "v_far"), ("depth_near", "v_near")]
        .map(|(label, entry)| device.depth_fill_pipeline(label, &fill_layout, &fill_shader, entry, samples))
}

pub(super) fn shader(device: &wgpu::Device, label: &str, source: String) -> wgpu::ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label:  label.into(),
        source: ShaderSource::Wgsl(source.into()),
    })
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::super::{MESH, SHADOW};
    use crate::render::data::MeshInstance;

    /// The float components the struct `name` of a shader carries over the
    /// stage boundary, `@builtin(position)` left out as docs/ios.md counts.
    fn components(source: &str, name: &str) -> u32 {
        let start = source.find(&format!("struct {name} {{")).expect("the struct is in the shader");
        let body = &source[start..source[start..].find('}').expect("the struct ends") + start];
        body.lines()
            .filter(|line| line.contains("@location"))
            .map(
                |line| match line.rsplit(": ").next().expect("a field has a type") {
                    ty if ty.starts_with("vec2") => 2,
                    ty if ty.starts_with("vec3") => 3,
                    ty if ty.starts_with("vec4") => 4,
                    _ => 1,
                },
            )
            .sum()
    }

    // An A7 draws nothing at all above 8, with no error, see docs/ios.md.
    #[test]
    fn every_mesh_shader_stays_in_the_room_of_an_a7() {
        assert_eq!(components(MESH, "VertexOutput"), 7);
        assert_eq!(components(MESH, "SmoothOutput"), 8);
        assert_eq!(components(MESH, "FlowOutput"), 8);
        assert_eq!(components(SHADOW, "CutoutOutput"), 4);
        assert_eq!(components(SHADOW, "SmoothCutoutOutput"), 2);
    }

    /// Every field of the struct `name` of a shader with its std430 offset.
    /// Only the types the instance holds are known, any other one panics.
    fn offsets(source: &str, name: &str) -> Vec<(String, usize)> {
        let start = source.find(&format!("struct {name} {{")).expect("the struct is in the shader");
        let body = &source[start..source[start..].find('}').expect("the struct ends") + start];
        let mut offset = 0;
        body.lines()
            .skip(1)
            .filter_map(|line| line.trim().strip_suffix(','))
            .map(|field| {
                let (field, ty) = field.split_once(": ").expect("a field has a type");
                let (align, size) = match ty {
                    "f32" | "u32" => (4, 4),
                    "vec4<f32>" | "vec4<u32>" => (16, 16),
                    "mat4x4<f32>" => (16, 64),
                    other => panic!("no std430 size for {other}"),
                };
                offset = usize::next_multiple_of(offset, align);
                let place = (field.to_string(), offset);
                offset += size;
                place
            })
            .collect()
    }

    // The instance is written by hand in Rust and in 2 shaders. A field
    // added to 1 of them shifted every field after it in the others, with
    // no error, and the shader read the wrong material.
    #[test]
    fn both_shaders_read_the_instance_the_rust_side_writes() {
        let normal = offset_of!(MeshInstance, normal);
        let expected: Vec<(String, usize)> = [
            ("model", offset_of!(MeshInstance, model)),
            ("normal0", normal),
            ("normal1", normal + 16),
            ("normal2", normal + 32),
            ("color", offset_of!(MeshInstance, color)),
            ("metallic", offset_of!(MeshInstance, metallic)),
            ("roughness", offset_of!(MeshInstance, roughness)),
            ("light_count", offset_of!(MeshInstance, light_count)),
            ("index", offset_of!(MeshInstance, index)),
            ("lights", offset_of!(MeshInstance, lights)),
            ("normal_scale", offset_of!(MeshInstance, normal_scale)),
            ("joint_base", offset_of!(MeshInstance, joint_base)),
            ("emissive", offset_of!(MeshInstance, emissive)),
            ("cutout", offset_of!(MeshInstance, cutout)),
            ("occlusion_strength", offset_of!(MeshInstance, occlusion_strength)),
            ("unlit", offset_of!(MeshInstance, unlit)),
            ("fade", offset_of!(MeshInstance, fade)),
            ("fade_flip", offset_of!(MeshInstance, fade_flip)),
            ("shallow", offset_of!(MeshInstance, shallow)),
        ]
        .map(|(field, offset)| (field.to_string(), offset))
        .into();
        assert_eq!(offsets(MESH, "MeshInstance"), expected, "mesh.wgsl");
        assert_eq!(offsets(SHADOW, "MeshInstance"), expected, "shadow.wgsl");
    }
}
