struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct BackgroundView {
    camera_pos: vec2<f32>,
    resolution: vec2<f32>,
    camera_rotation: f32,
    scale: f32,
    _padding:   vec2<u32>,
}

@group(0) @binding(0) var<uniform> view: BackgroundView;

@vertex
fn v_main(
    vertex: Vertex,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex.uv;
    out.pos = vec4<f32>(vertex.pos + view.camera_pos, 0.9, 1.0);

    out.pos.x *= view.resolution.y / view.resolution.x;

    out.pos.x *= view.scale;
    out.pos.y *= view.scale;

    let scale: f32 = view.resolution.y / 20.0;

    out.pos.x /= scale;
    out.pos.y /= scale;

    return out;
}

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
