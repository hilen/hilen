
// Must match `TerrainDraw` in `terrain_pipeline.rs`. A fill, kind 0,
// repeats its image in level space. A surface strip, kind 1, repeats it
// along x and takes its v from the vertex, 0 on the ground outline and 1
// at the strip bottom. An edge line, kind 2, is the flat `color`. A blend
// band, kind 3, repeats its image like a fill and keeps a texel only where
// the 4 by 4 dither threshold of that texel is over v, 0 on the border and
// 1 at the far side, so the spill thins out in whole texels. A surface
// end, kind 4, takes both u and v from the vertex.
struct TerrainDraw {
    uv_scale: vec2<f32>,
    z:        f32,
    kind:     u32,
    color:    vec4<f32>,
}

@group(1) @binding(0)
var<uniform> draw: TerrainDraw;

// Four float components cross to the fragment stage, under the A7 limit
// in docs/ios.md.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) v: f32,
    @location(2) u: f32,
}

@vertex
fn v_main(
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos   = level_to_clip(pos, draw.z);
    out.world = pos;
    out.v     = uv.y;
    out.u     = uv.x;
    return out;
}

@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = vec2<f32>(in.world.x * draw.uv_scale.x, -in.world.y * draw.uv_scale.y);
    if draw.kind == 4u {
        let half_texel = 0.5 / vec2<f32>(textureDimensions(t_diffuse));
        uv = clamp(vec2<f32>(in.u, in.v), half_texel, 1.0 - half_texel);
    }
    if draw.kind == 1u {
        // Under MSAA a pixel the strip edge only partly covers runs at its
        // center, off the strip, and the repeating sampler would draw the
        // opposite edge of the image there.
        let half_texel = 0.5 / f32(textureDimensions(t_diffuse).y);
        uv.y = clamp(in.v, half_texel, 1.0 - half_texel);
    }
    // Sampled before the branch, a texture read needs uniform control flow.
    let sampled = textureSample(t_diffuse, s_diffuse, uv);
    let tex = select(sampled, draw.color, draw.kind == 2u);
    if draw.kind == 3u && dither(uv) <= in.v {
        discard;
    }
    if tex.a < 0.004 {
        discard;
    }
    return vec4<f32>(tex.rgb * sprite_light(in.world), tex.a);
}

// The 4 by 4 ordered dither threshold of the texel under `uv`, from 1/32
// to 31/32.
fn dither(uv: vec2<f32>) -> f32 {
    var bayer = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);
    let size = vec2<f32>(textureDimensions(t_diffuse));
    let texel = vec2<u32>(floor(fract(uv) * size));
    return (f32(bayer[(texel.y % 4u) * 4u + texel.x % 4u]) + 0.5) / 16.0;
}
