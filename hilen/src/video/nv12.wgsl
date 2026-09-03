// NV12 planes to RGB. The luma plane and the half size interleaved chroma
// plane come straight from the decoder, one fullscreen triangle writes the
// RGBA image the `ImageView` draws. Output is encoded sRGB, which is what a
// BT.709 or BT.601 video carries, so it lands in the frame unchanged.

struct Params {
    full_range: u32,
    bt601: u32,
    padding: vec2<u32>,
}

@group(0) @binding(0) var t_y: texture_2d<f32>;
@group(0) @binding(1) var t_uv: texture_2d<f32>;
@group(0) @binding(2) var s_planes: sampler;

@group(1) @binding(0) var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn v_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index & 2u) * 2 - 1);
    var out: VertexOutput;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var y = textureSample(t_y, s_planes, in.uv).r;
    let chroma = textureSample(t_uv, s_planes, in.uv).rg;
    var u = chroma.x - 0.5;
    var v = chroma.y - 0.5;

    if params.full_range == 0u {
        y = (y - 16.0 / 255.0) * (255.0 / 219.0);
        u = u * (255.0 / 224.0);
        v = v * (255.0 / 224.0);
    }

    var rgb: vec3<f32>;
    if params.bt601 == 1u {
        rgb = vec3<f32>(y + 1.402 * v, y - 0.344136 * u - 0.714136 * v, y + 1.772 * u);
    } else {
        rgb = vec3<f32>(y + 1.5748 * v, y - 0.187324 * u - 0.468124 * v, y + 1.8556 * u);
    }

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
