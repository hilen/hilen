struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
}

// Fullscreen triangle, the scissor and the stencil of the pass cut it.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let raw = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.pos = vec4<f32>(raw.x * 2.0 - 1.0, 1.0 - raw.y * 2.0, 0.0, 1.0);
    return out;
}

struct GroupParams {
    opacity: f32,
    _padding_0: f32,
    _padding_1: f32,
    _padding_2: f32,
}

@group(0) @binding(0) var group_image: texture_2d<f32>;
@group(0) @binding(1) var group_sampler: sampler;

@group(1) @binding(0) var<uniform> params: GroupParams;

// The group image has the size of the frame, so a fragment reads the pixel
// it sits on. The image holds colors already multiplied by their alpha, the
// blend of the pipeline expects that, and the fade scales all 4 channels.
@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.pos.xy / vec2<f32>(textureDimensions(group_image));
    return textureSampleLevel(group_image, group_sampler, uv, 0.0) * params.opacity;
}
