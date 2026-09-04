
struct RectView {
    resolution: vec2<f32>,
    _padding: vec2<u32>,
}

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

// Only what the vertex stage needs. The rest is read from `instances`.
struct UIImageVertex {
    @location(4) position:      vec2<f32>,
    @location(5) size:          vec2<f32>,
    @location(6) uv_position:   vec2<f32>,
    @location(7) uv_size:       vec2<f32>,
    @location(9) z_position:    f32,
    @location(10) flags:        u32,
    @location(11) scale:        f32,
}

// Field order and offsets are `std430` and must match `UIImageInstance`, which
// has a test pinning them.
struct UIImageInstance {
    border_color: vec4<f32>,
    corner_radii: vec4<f32>,
    position: vec2<f32>,
    size: vec2<f32>,
    uv_position: vec2<f32>,
    uv_size: vec2<f32>,
    border_width: f32,
    z_position: f32,
    flags: u32,
    scale: f32,
}

@group(0) @binding(0)
var<uniform> view: RectView;

@group(1) @binding(0)
var<storage, read> instances: array<UIImageInstance>;

// An A7 GPU draws nothing at all from a shader carrying more than eight float
// components between the stages, see docs/ios.md. Only `uv` and `corner_uv`
// really vary across the shape, so everything else is read from `instances`.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) corner_uv: vec2<f32>,
    @location(2) @interpolate(flat) index: u32,
}

@vertex
fn v_main(
    model: Vertex,
    instance: UIImageVertex,
    @builtin(instance_index) index: u32,
) -> VertexOutput {
    let flip_x: bool = ((instance.flags >> 0u) & 1u) != 0u;
    let flip_y: bool = ((instance.flags >> 1u) & 1u) != 0u;

    var pos = model.pos;

    if flip_x {
        pos.x *= -1.0;
    }

    if flip_y {
        pos.y *= -1.0;
    }

    var out_pos: vec4<f32> = vec4<f32>(pos, instance.z_position, 1.0);

    out_pos.y = -out_pos.y;

    out_pos.x /= 2.0;
    out_pos.y /= 2.0;

    out_pos.x += 0.5;
    out_pos.y += 0.5;

    out_pos.x /= view.resolution.x;
    out_pos.y /= view.resolution.y;

    out_pos.x *= instance.size.x * instance.scale;
    out_pos.y *= instance.size.y * instance.scale;

    out_pos.x += instance.position.x * instance.scale / view.resolution.x;
    out_pos.y += instance.position.y * instance.scale / view.resolution.y;

    out_pos.y *= -1.0;

    out_pos.x -= 0.5;
    out_pos.y += 0.5;

    out_pos.x *= 2.0;
    out_pos.y *= 2.0;

    var out: VertexOutput;
    out.pos = out_pos;
    out.uv  = instance.uv_position + model.uv * instance.uv_size;
    // Screen orientation, not texture orientation: x follows the
    // flipped pos, y is negated to undo the extra negation above,
    // so negative y is the top like in the other UI shaders.
    out.corner_uv = vec2<f32>(pos.x, -pos.y) * 0.5;
    out.index = index;
    return out;
}

@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;

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

// Signed distance to a rounded box centered at the origin. Negative
// inside. With radius 0 the inside distance is minus the distance to
// the nearest edge, so the border band below works for square corners.
fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// One pixel wide analytic edge coverage. See ui_rect.wgsl.
fn edge_coverage(dist: f32, width: f32) -> f32 {
    return clamp(0.5 - dist / width, 0.0, 1.0);
}

// The distance change per screen pixel, zero guarded, see ui_rect.wgsl.
fn pixel_width(derivative: f32, scale: f32) -> f32 {
    return select(1.0 / scale, derivative, derivative > 0.0);
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIImageInstance = instances[in.index];

    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    let local_pos: vec2<f32> = in.corner_uv * instance.size;
    let radius: f32 = pick_radius(local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(local_pos, instance.size * 0.5, radius);

    // Zero guarded, see ui_rect.wgsl.
    let width: f32 = pixel_width(fwidth(dist), instance.scale);

    let coverage: f32 = edge_coverage(dist, width);

    var rgb: vec3<f32> = tex.rgb;
    var alpha: f32 = tex.a;

    if instance.border_width > 0.0 {
        let fill: f32 = edge_coverage(dist + instance.border_width, width);
        rgb = mix(instance.border_color.rgb, tex.rgb, fill);
        alpha = mix(instance.border_color.a, tex.a, fill);
    }

    alpha *= coverage;

    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(rgb, alpha);
}
