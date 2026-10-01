@group(0) @binding(0)
var t_frame: texture_2d<f32>;
@group(0) @binding(1)
var s_frame: sampler;

struct PictureOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// One triangle over the whole target.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> PictureOutput {
    var out: PictureOutput;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    out.pos = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

// The frame over a clear background holds its color times its coverage,
// from the multisample resolve and from every blended node. An image is
// drawn with straight alpha, so the color is divided back out.
@fragment
fn f_main(in: PictureOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_frame, s_frame, in.uv);
    if color.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(color.rgb / color.a, color.a);
}
