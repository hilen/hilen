struct RectView {
    resolution: vec2<f32>,
    _padding: vec2<u32>,
}

// Field order and offsets are `std430` and must match `UIRectInstance`, which
// has a test pinning them.
struct UIRectInstance {
    color: vec4<f32>,
    border_color: vec4<f32>,
    corner_radii: vec4<f32>,
    position: vec2<f32>,
    size: vec2<f32>,
    border_width: f32,
    z_position: f32,
    scale: f32,
}

@group(0) @binding(0)
var<uniform> view: RectView;

@group(1) @binding(0)
var<storage, read> instances: array<UIRectInstance>;

// An A7 GPU draws nothing at all from a shader carrying more than eight float
// components between the stages. It builds and validates, then produces no
// pixels and no error, see docs/ios.md. Everything the fragment needs is
// constant across the shape, so it reads that from `instances` and only `uv`
// crosses the boundary. Keep this struct small.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>,
    @location(5) position: vec2<f32>,
    @location(6) size: vec2<f32>,
    @location(8) z_position: f32,
    @location(9) scale: f32,
    @builtin(instance_index) index: u32,
) -> VertexOutput {
    var out_pos: vec4<f32> = vec4<f32>(model, z_position, 1.0);

    out_pos.x /= 2.0;
    out_pos.y /= 2.0;

    out_pos.x += 0.5;
    out_pos.y += 0.5;

    out_pos.x /= view.resolution.x;
    out_pos.y /= view.resolution.y;

    out_pos.x *= size.x * scale;
    out_pos.y *= size.y * scale;

    out_pos.x += position.x * scale / view.resolution.x;
    out_pos.y += position.y * scale / view.resolution.y;

    out_pos.y *= -1.0;

    out_pos.x -= 0.5;
    out_pos.y += 0.5;

    out_pos.x *= 2.0;
    out_pos.y *= 2.0;

    var out: VertexOutput;
    out.pos = out_pos;
    out.uv = model * 0.5;
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

// Signed distance to a rounded box centered at the origin. Negative
// inside. With radius 0 the inside distance is minus the distance to
// the nearest edge, so the border band below works for square corners.
fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Coverage of the pixel by an edge `dist` away, given the distance change per
// screen pixel in `width`. One pixel wide analytic anti aliasing: 1 well
// inside, 0 well outside, a smooth ramp across the boundary.
fn edge_coverage(dist: f32, width: f32) -> f32 {
    return clamp(0.5 - dist / width, 0.0, 1.0);
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIRectInstance = instances[in.index];

    let size: vec2<f32> = instance.size;
    let local_pos: vec2<f32> = in.uv * size;
    let radius: f32 = pick_radius(local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(local_pos, size * 0.5, radius);

    // One derivative for the whole shader. A `fwidth` inside a branch sits in
    // non uniform control flow, where the result is undefined.
    let width: f32 = fwidth(dist);

    let coverage: f32 = edge_coverage(dist, width);

    var rgb: vec3<f32> = instance.color.rgb;
    var alpha: f32 = instance.color.a;

    if instance.border_width > 0.0 {
        // 1 in the fill interior, 0 in the border band, one pixel ramp
        // at the fill to border boundary.
        let fill: f32 = edge_coverage(dist + instance.border_width, width);
        rgb = mix(instance.border_color.rgb, instance.color.rgb, fill);
        alpha = mix(instance.border_color.a, instance.color.a, fill);
    }

    alpha *= coverage;

    // A fully transparent fragment must not write depth, or it would
    // occlude whatever is drawn after it inside the same shape.
    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(rgb, alpha);
}
