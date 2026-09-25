
struct PolygonView {
    color:    vec4<f32>,
    pos:      vec2<f32>,
    rot:      f32,
    _padding: u32,
}

@group(1) @binding(0)
var<uniform> polygon_view: PolygonView;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>
) -> VertexOutput {
    let local = (vec4<f32>(model, 0.0, 1.0) * rotation_z_matrix(-polygon_view.rot)).xy;
    let world = local + polygon_view.pos;

    var out: VertexOutput;
    out.pos   = level_to_clip(world, 0.8);
    out.world = world;
    return out;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(polygon_view.color.rgb * sprite_light(in.world), polygon_view.color.a);
}
