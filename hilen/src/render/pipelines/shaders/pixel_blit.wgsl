// Shows the art resolution texture of a pixel art level over the level
// area. Each screen pixel reads the texel under it with the nearest filter,
// so an art pixel stays a hard square however many screen pixels it covers.
// Must match `PixelBlit` in `pixel_blit_pipeline.rs`.
struct PixelBlit {
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    z:      f32,
    pad_0:  f32,
    pad_1:  f32,
    pad_2:  f32,
}

@group(0) @binding(0)
var<uniform> blit: PixelBlit;

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// A strip of 4 corners over the whole viewport, no vertex buffer.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let corner = vec2<f32>(f32(index & 1u), f32(index >> 1u));
    var out: VertexOutput;
    out.pos = vec4<f32>(corner.x * 2.0 - 1.0, 1.0 - corner.y * 2.0, blit.z, 1.0);
    out.uv  = mix(blit.uv_min, blit.uv_max, corner);
    return out;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
