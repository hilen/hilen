
struct RectView {
    resolution: vec2<f32>,
    _padding: vec2<u32>,
}

// Only what the vertex stage needs. The rest is read from `instances`.
struct UIGradientVertex {
    @location(2) position:      vec2<f32>,
    @location(3) size:          vec2<f32>,
    @location(7) z_position:    f32,
    @location(8) scale:         f32,
}

// Field order and offsets are `std430` and must match `UIGradientInstance`,
// which has a test pinning them.
struct UIGradientInstance {
    position: vec2<f32>,
    size: vec2<f32>,
    start_color: vec4<f32>,
    end_color: vec4<f32>,
    corner_radii: vec4<f32>,
    z_position: f32,
    scale: f32,
    linear_axis: vec2<f32>,
    radial_center: vec2<f32>,
    radial_radii: vec2<f32>,
    linear_bias: f32,
    kind: u32,
    border_width: f32,
    border_color: vec4<f32>,
}

const GRADIENT_RADIAL: u32 = 1u;

@group(0) @binding(0)
var<uniform> view: RectView;

@group(1) @binding(0)
var<storage, read> instances: array<UIGradientInstance>;

// An A7 GPU draws nothing at all from a shader carrying more than eight float
// components between the stages, see docs/ios.md. Only `uv` really varies
// across the shape, so the colors and the ramp shape are read from
// `instances` and the ramp itself is computed in the fragment stage.
struct VertexOutput {
    @builtin(position) pos:   vec4<f32>,
          @location(0) uv:   vec2<f32>,
          @location(1) @interpolate(flat) index: u32,
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>,
    instance: UIGradientVertex,
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
    out.pos   = out_pos;

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

// Signed distance to a rounded box centered at the origin. Negative inside.
fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// One pixel wide analytic edge coverage, see ui_rect.wgsl.
fn edge_coverage(dist: f32, width: f32) -> f32 {
    return clamp(0.5 - dist / width, 0.0, 1.0);
}

// The distance change per screen pixel, zero guarded, see ui_rect.wgsl.
fn pixel_width(derivative: f32, scale: f32) -> f32 {
    return select(1.0 / scale, derivative, derivative > 0.0);
}

// How far along the ramp this pixel is, 0 at the start color and 1 at the
// end. The CSS math is baked into the instance, so a linear gradient costs
// one dot product and a radial one length.
fn gradient_ramp(uv: vec2<f32>, instance: UIGradientInstance) -> f32 {
    var ramp: f32;

    if instance.kind == GRADIENT_RADIAL {
        ramp = length((uv - instance.radial_center) / instance.radial_radii);
    } else {
        ramp = instance.linear_bias + dot(uv, instance.linear_axis);
    }

    return clamp(ramp, 0.0, 1.0);
}

// CSS interpolates a gradient with premultiplied alpha, so a stop fading to
// `transparent` keeps its hue instead of sliding towards black.
fn gradient_color(instance: UIGradientInstance, ramp: f32) -> vec4<f32> {
    let start = vec4<f32>(instance.start_color.rgb * instance.start_color.a, instance.start_color.a);
    let end = vec4<f32>(instance.end_color.rgb * instance.end_color.a, instance.end_color.a);

    let color = mix(start, end, ramp);

    // The pipeline blends with plain alpha, so the color goes back to
    // straight alpha here. A fully transparent pixel is discarded below.
    return vec4<f32>(color.rgb / max(color.a, 0.0001), color.a);
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIGradientInstance = instances[in.index];

    let color = gradient_color(instance, gradient_ramp(in.uv, instance));

    let local_pos: vec2<f32> = in.uv * instance.size;
    let radius: f32 = pick_radius(local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(local_pos, instance.size * 0.5, radius);

    // Zero guarded, see ui_rect.wgsl.
    let width: f32 = pixel_width(fwidth(dist), instance.scale);

    var rgb: vec3<f32> = color.rgb;
    var alpha: f32 = color.a;

    if instance.border_width > 0.0 {
        // 1 in the ramp interior, 0 in the border band, one pixel ramp at
        // the boundary between them.
        let fill: f32 = edge_coverage(dist + instance.border_width, width);
        rgb = mix(instance.border_color.rgb, color.rgb, fill);
        alpha = mix(instance.border_color.a, color.a, fill);
    }

    alpha *= edge_coverage(dist, width);

    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(rgb, alpha);
}
