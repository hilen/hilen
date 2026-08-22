struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle. uv y points down, matching how the scene
// texture is written, so every pass keeps screen orientation.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let raw = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.pos = vec4<f32>(raw.x * 2.0 - 1.0, 1.0 - raw.y * 2.0, 0.0, 1.0);
    out.uv = raw;
    return out;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn f_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(source, source_sampler, in.uv, 0.0);
}

struct BlurParams {
    direction: vec2<f32>,
    sigma: f32,
    _padding: f32,
}

@group(1) @binding(0) var<uniform> params: BlurParams;

// One direction of a separable gaussian. Weights are computed in
// place and normalized by their sum, so any tap count stays energy
// preserving. The tap count is uniform across fragments, which keeps
// the loop valid for sampling.
@fragment
fn f_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = params.direction / vec2<f32>(textureDimensions(source));
    let taps = i32(min(ceil(params.sigma * 3.0), 32.0));

    var sum = textureSampleLevel(source, source_sampler, in.uv, 0.0);
    var weights = 1.0;

    for (var i = 1; i <= taps; i++) {
        let offset = f32(i);
        let weight = exp(-0.5 * (offset / params.sigma) * (offset / params.sigma));
        sum += (textureSampleLevel(source, source_sampler, in.uv + texel * offset, 0.0)
            + textureSampleLevel(source, source_sampler, in.uv - texel * offset, 0.0))
            * weight;
        weights += 2.0 * weight;
    }

    return sum / weights;
}
