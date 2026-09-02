// Field order and offsets are std140 and must match `PathView`, which
// has a test pinning them.
struct PathView {
    position: vec2<f32>,
    resolution: vec2<f32>,
    gradient_a: vec2<f32>,
    gradient_b: vec2<f32>,
    colors: array<vec4<f32>, 8>,
    // The stop positions, packed as two vec4 so std140 does not pad
    // each float to 16 bytes.
    positions0: vec4<f32>,
    positions1: vec4<f32>,
    z_position: f32,
    scale: f32,
    kind: u32,
    stop_count: u32,
    grain: f32,
}

const KIND_LINEAR: u32 = 1u;
const KIND_RADIAL: u32 = 2u;
const KIND_CONIC: u32 = 3u;

const TAU: f32 = 6.2831853;

@group(0) @binding(0) var<uniform> path_view: PathView;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    // The untransformed path point, so the ramp math runs in the same
    // view space the paint's points were given in.
    @location(0) local: vec2<f32>,
}

@vertex
fn v_main(
    @location(0) vertex: vec2<f32>,
) -> VertexOutput {
    let p = (vertex + path_view.position) * path_view.scale;

    let x = p.x * 2.0 / path_view.resolution.x - 1.0;
    let y = 1.0 - p.y * 2.0 / path_view.resolution.y;

    var out: VertexOutput;
    out.pos = vec4<f32>(x, y, path_view.z_position, 1.0);
    out.local = vertex;
    return out;
}

fn stop_position(i: u32) -> f32 {
    if i < 4u {
        return path_view.positions0[i];
    }
    return path_view.positions1[i - 4u];
}

// How far along the ramp this pixel is, 0 at the first stop's home and
// 1 at the last's. The conic ramp is a triangle wave over the angle,
// so it runs there and back `repeats` times per turn and stays
// seamless.
fn ramp(local: vec2<f32>) -> f32 {
    if path_view.kind == KIND_LINEAR {
        let axis = path_view.gradient_b - path_view.gradient_a;
        return clamp(dot(local - path_view.gradient_a, axis) / dot(axis, axis), 0.0, 1.0);
    }
    if path_view.kind == KIND_RADIAL {
        return clamp(length(local - path_view.gradient_a) / path_view.gradient_b.x, 0.0, 1.0);
    }
    if path_view.kind == KIND_CONIC {
        let d = local - path_view.gradient_a;
        let turns = atan2(d.y, d.x) / TAU * path_view.gradient_b.x;
        return abs(fract(turns) * 2.0 - 1.0);
    }
    return 0.0;
}

// CSS interpolates a gradient with premultiplied alpha, so a stop
// fading to transparent keeps its hue instead of sliding towards
// black. Between its two neighboring stops each pixel mixes those two
// alone.
fn ramp_color(t: f32) -> vec4<f32> {
    var color = path_view.colors[0];
    var previous = stop_position(0u);
    color = vec4<f32>(color.rgb * color.a, color.a);

    for (var i = 1u; i < path_view.stop_count; i = i + 1u) {
        let next = stop_position(i);
        var stop = path_view.colors[i];
        stop = vec4<f32>(stop.rgb * stop.a, stop.a);
        let segment = clamp((t - previous) / max(next - previous, 0.0001), 0.0, 1.0);
        color = mix(color, stop, segment);
        previous = next;
    }
    return color;
}

// An integer hash, bit exact on every GPU. The float hash it replaces,
// fract(sin(x) * 43758.5453), fed sin arguments of tens of thousands of
// radians, and drivers round sin that large differently, so the grain
// was one pattern on Metal and another on Mesa and no recorded pixel
// could hold on both.
fn pcg(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// The grain seed is quantized before hashing, so the only float work
// left is a floor, which every driver agrees on away from cell edges.
// On a conic ramp it follows the angle alone, so the noise streaks
// along the radius like machined metal.
fn grain(local: vec2<f32>) -> f32 {
    var seed: vec2<i32>;
    if path_view.kind == KIND_CONIC {
        let d = local - path_view.gradient_a;
        seed = vec2<i32>(i32(floor(atan2(d.y, d.x) * 1000.0)), 0);
    } else {
        // Quarter points, one cell per pixel up to a 4x display scale.
        seed = vec2<i32>(floor(local * 4.0));
    }
    let bits = bitcast<vec2<u32>>(seed);
    let hash = pcg(bits.x ^ pcg(bits.y));
    return f32(hash & 0xffffu) / 65535.0 - 0.5;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = ramp_color(ramp(in.local));

    if path_view.grain != 0.0 {
        color = vec4<f32>(color.rgb * (1.0 + grain(in.local) * path_view.grain), color.a);
    }

    // An invisible fragment must not write depth, it would mask what
    // draws behind this path later, the trap the rect shader also
    // discards its way out of.
    if color.a < 0.002 {
        discard;
    }
    // The pipeline blends with plain alpha, so the premultiplied mix
    // goes back to straight alpha here.
    return vec4<f32>(color.rgb / max(color.a, 0.0001), color.a);
}
