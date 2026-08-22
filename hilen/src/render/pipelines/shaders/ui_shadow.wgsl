struct RectView {
    resolution: vec2<f32>,
    _padding: vec2<u32>,
}

// Only what the vertex stage needs. The rest is read from `instances`.
struct UIShadowVertex {
    @location(2) position:      vec2<f32>,
    @location(3) size:          vec2<f32>,
    @location(6) blur:          f32,
    @location(7) z_position:    f32,
    @location(8) scale:         f32,
}

// Field order and offsets are `std430` and must match `UIShadowInstance`, which
// has a test pinning them.
struct UIShadowInstance {
    position: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    corner_radii: vec4<f32>,
    blur: f32,
    z_position: f32,
    scale: f32,
}

@group(0) @binding(0)
var<uniform> view: RectView;

@group(1) @binding(0)
var<storage, read> instances: array<UIShadowInstance>;

// An A7 GPU draws nothing at all from a shader carrying more than eight float
// components between the stages, see docs/ios.md. Only `local_pos` really
// varies across the shape, so the rest is read from `instances`.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
}

// The quad is the casting rect expanded by blur on every side, so the
// falloff has room outside the rect.
@vertex
fn v_main(
    @location(0) model: vec2<f32>,
    instance: UIShadowVertex,
    @builtin(instance_index) index: u32,
) -> VertexOutput {
    let expanded: vec2<f32> = instance.size + vec2<f32>(instance.blur * 2.0);
    let origin: vec2<f32> = instance.position - vec2<f32>(instance.blur);

    var out_pos: vec4<f32> = vec4<f32>(model, instance.z_position, 1.0);

    out_pos.x /= 2.0;
    out_pos.y /= 2.0;

    out_pos.x += 0.5;
    out_pos.y += 0.5;

    out_pos.x /= view.resolution.x;
    out_pos.y /= view.resolution.y;

    out_pos.x *= expanded.x * instance.scale;
    out_pos.y *= expanded.y * instance.scale;

    out_pos.x += origin.x * instance.scale / view.resolution.x;
    out_pos.y += origin.y * instance.scale / view.resolution.y;

    out_pos.y *= -1.0;

    out_pos.x -= 0.5;
    out_pos.y += 0.5;

    out_pos.x *= 2.0;
    out_pos.y *= 2.0;

    var out: VertexOutput;
    out.pos = out_pos;
    out.local_pos = model * 0.5 * expanded;
    out.index = index;

    return out;
}

// Radii order: top left, top right, bottom left, bottom right.
// Local coordinates have negative y at the top.
fn pick_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    if p.y < 0.0 {
        if p.x < 0.0 {
            return radii.x;
        }
        return radii.y;
    }
    if p.x < 0.0 {
        return radii.z;
    }
    return radii.w;
}

// Signed distance to a rounded box centered at the origin. Negative inside.
fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIShadowInstance = instances[in.index];

    let radius: f32 = pick_radius(in.local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(in.local_pos, instance.size * 0.5, radius);

    let alpha: f32 = instance.color.a * (1.0 - smoothstep(-instance.blur, instance.blur, dist));

    // Skip depth writes on the invisible outer band of the quad.
    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(instance.color.rgb, alpha);
}
