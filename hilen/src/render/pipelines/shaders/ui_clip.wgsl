// The rounded clip mask. Draws no color, only the stencil, where the
// pipeline's stencil op moves the value inside the rounded box. See
// `UIClipPipeline`.

struct ClipView {
    resolution: vec2<f32>,
    // The coverage a fragment needs to count as inside. Near zero with
    // multisampling, where alpha to coverage grades the edge per sample,
    // one half without it.
    threshold: f32,
    _padding: u32,
}

// Same layout as `UIRectInstance`, see ui_rect.wgsl. Only the position,
// size and radii matter here.
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
var<uniform> view: ClipView;

@group(1) @binding(0)
var<storage, read> instances: array<UIRectInstance>;

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

fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn edge_coverage(dist: f32, width: f32) -> f32 {
    return clamp(0.5 - dist / width, 0.0, 1.0);
}

// The distance change per screen pixel, zero guarded, see ui_rect.wgsl.
fn pixel_width(derivative: f32, scale: f32) -> f32 {
    return select(1.0 / scale, derivative, derivative > 0.0);
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance: UIRectInstance = instances[in.index];

    let size: vec2<f32> = instance.size;
    let local_pos: vec2<f32> = in.uv * size;
    let radius: f32 = pick_radius(local_pos, instance.corner_radii);
    let dist: f32 = rounded_box_sdf(local_pos, size * 0.5, radius);

    // Zero guarded, see ui_rect.wgsl.
    let width: f32 = pixel_width(fwidth(dist), instance.scale);
    let coverage: f32 = edge_coverage(dist, width);

    if coverage < view.threshold {
        discard;
    }

    return vec4<f32>(0.0, 0.0, 0.0, coverage);
}
