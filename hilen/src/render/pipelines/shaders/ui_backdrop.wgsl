struct RectView {
    resolution: vec2<f32>,
    _padding: vec2<u32>,
}

// Only what the vertex stage needs. Locations follow `UIRectInstance`, which is
// shared with `ui_rect.wgsl`, so they move with that struct's field order.
struct UIRectVertex {
    @location(5) position:      vec2<f32>,
    @location(6) size:          vec2<f32>,
    @location(8) z_position:    f32,
    @location(9) scale:         f32,
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

@group(1) @binding(0) var blurred: texture_2d<f32>;
@group(1) @binding(1) var blurred_sampler: sampler;

@group(2) @binding(0)
var<storage, read> instances: array<UIRectInstance>;

// An A7 GPU draws nothing at all from a shader carrying more than eight float
// components between the stages, see docs/ios.md. Only `uv` really varies
// across the shape, so the rest is read from `instances`.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>,
    instance: UIRectVertex,
    @builtin(instance_index) index: u32,
) -> VertexOutput {
    var out_pos: vec4<f32> = vec4<f32>(model, instance.z_position, 1.0);

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

// One pixel wide analytic edge coverage. See ui_rect.wgsl.
fn edge_coverage(dist: f32, width: f32) -> f32 {
    return clamp(0.5 - dist / width, 0.0, 1.0);
}

// The distance change per screen pixel, zero guarded, see ui_rect.wgsl.
fn pixel_width(derivative: f32, scale: f32) -> f32 {
    return select(1.0 / scale, derivative, derivative > 0.0);
}

// The blurred scene is sampled at the fragment's own screen position,
// so the quad shows the blur of exactly what sits under it. The
// instance color is a tint mixed on top by its alpha.
@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIRectInstance = instances[in.index];

    let local_pos: vec2<f32> = in.uv * instance.size;
    let radius: f32 = pick_radius(local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(local_pos, instance.size * 0.5, radius);

    // Zero guarded, see ui_rect.wgsl.
    let width: f32 = pixel_width(fwidth(dist), instance.scale);

    let coverage: f32 = edge_coverage(dist, width);
    if coverage < 0.004 {
        discard;
    }

    let screen_uv = in.pos.xy / view.resolution;
    let blur = textureSampleLevel(blurred, blurred_sampler, screen_uv, 0.0);
    var rgb: vec3<f32> = mix(blur.rgb, instance.color.rgb, instance.color.a);

    if instance.border_width > 0.0 {
        let fill: f32 = edge_coverage(dist + instance.border_width, width);
        rgb = mix(instance.border_color.rgb, rgb, fill);
    }

    return vec4<f32>(rgb, coverage);
}
