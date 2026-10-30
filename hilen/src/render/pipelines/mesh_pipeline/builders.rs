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
        device_helper::{DeviceHelper, MeshDraw, SCENE_DEPTH_COMPARE},
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
    pub(super) fn new(device: &wgpu::Device, layout: &PipelineLayout, shader: &ShaderModule) -> Self {
        let pipeline = |label: &str, draw: MeshDraw| smooth_pipeline(device, label, layout, shader, draw);
        Self {
            opaque:      pipeline("mesh_smooth_pipeline", MeshDraw::SmoothOpaque),
            cutout:      pipeline("mesh_smooth_cutout_pipeline", MeshDraw::SmoothCutout),
            translucent: pipeline("mesh_smooth_translucent_pipeline", MeshDraw::SmoothTranslucent),
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
) -> RenderPipeline {
    device.mesh_pipeline(label, layout, shader, STATIC_LAYOUTS, "v_smooth", draw)
}

/// The thin lines, then the wide lines that are depth tested and the wide
/// lines on top of everything.
pub(super) fn line_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    lines: &wgpu::ShaderModule,
    wide: &wgpu::ShaderModule,
) -> [RenderPipeline; 3] {
    [
        device.pipeline(
            "lines_pipeline",
            layout,
            lines,
            SCENE_DEPTH_COMPARE,
            PrimitiveTopology::LineList,
            LINE_LAYOUTS,
        ),
        device.pipeline(
            "wide_lines_tested",
            layout,
            wide,
            SCENE_DEPTH_COMPARE,
            PrimitiveTopology::TriangleList,
            WIDE_LINE_LAYOUTS,
        ),
        device.pipeline(
            "wide_lines_on_top",
            layout,
            wide,
            CompareFunction::Always,
            PrimitiveTopology::TriangleList,
            WIDE_LINE_LAYOUTS,
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
) -> [RenderPipeline; 2] {
    [
        device.mesh_pipeline(
            &format!("{name}_pipeline"),
            layout,
            shader,
            STATIC_LAYOUTS,
            "v_main",
            draw,
        ),
        device.mesh_pipeline(
            &format!("{name}_skinned_pipeline"),
            layout,
            shader,
            SKINNED_LAYOUTS,
            "v_skinned",
            draw,
        ),
    ]
}

/// The grass of the mesh shader from `v_grass`, the depth of its blades
/// and then their light.
pub(super) fn grass_pipelines(
    device: &wgpu::Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> [RenderPipeline; 2] {
    [
        ("grass_depth_pipeline", MeshDraw::GrassDepth),
        ("grass_pipeline", MeshDraw::Grass),
    ]
    .map(|(label, draw)| device.mesh_pipeline(label, layout, shader, GRASS_LAYOUTS, "v_grass", draw))
}

/// The 2 passes that write the far end of the scene's depth before it
/// draws and the near end after, see `fill_depth`.
pub(super) fn depth_fill_pipelines(device: &wgpu::Device) -> [RenderPipeline; 2] {
    let fill_shader = shader(device, "depth_fill_shader", DEPTH_FILL.to_string());
    let fill_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label:              "depth_fill_layout".into(),
        bind_group_layouts: &[],
        immediate_size:     0,
    });
    [("depth_far", "v_far"), ("depth_near", "v_near")]
        .map(|(label, entry)| device.depth_fill_pipeline(label, &fill_layout, &fill_shader, entry))
}

pub(super) fn shader(device: &wgpu::Device, label: &str, source: String) -> wgpu::ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label:  label.into(),
        source: ShaderSource::Wgsl(source.into()),
    })
}

#[cfg(test)]
mod test {
    use super::super::{MESH, SHADOW};

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
        assert_eq!(components(SHADOW, "CutoutOutput"), 4);
        assert_eq!(components(SHADOW, "SmoothCutoutOutput"), 2);
    }
}
