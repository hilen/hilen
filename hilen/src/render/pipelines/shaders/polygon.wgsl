
struct SpriteView {
    camera_pos: vec2<f32>,
    resolution: vec2<f32>,
    camera_rotation: f32,
    scale: f32,
    _padding: vec2<u32>,
}

struct PolygonView {
    color:    vec4<f32>,
    pos:      vec2<f32>,
    rot:      f32,
    _padding: u32,
}

@group(0) @binding(0)
var<uniform> view: SpriteView;

@group(1) @binding(0)
var<uniform> polygon_view: PolygonView;


fn rotation_z_matrix(angle: f32) -> mat4x4<f32> {
    let cos_z: f32 = cos(angle);
    let sin_z: f32 = sin(angle);
    return mat4x4<f32>(
        vec4<f32>(cos_z, sin_z, 0.0, 0.0),
        vec4<f32>(-sin_z, cos_z, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>
) -> @builtin(position) vec4<f32> {
    var out: vec4<f32> = vec4<f32>(model, 0.8, 1.0);

//    out.x *= instance.size.x;
//    out.y *= instance.size.y;

    out *= rotation_z_matrix(-polygon_view.rot);

    out.x += polygon_view.pos.x - view.camera_pos.x;
    out.y += polygon_view.pos.y - view.camera_pos.y;

    out *=  rotation_z_matrix(view.camera_rotation);

    out.x *= view.resolution.y / view.resolution.x;

    out.x *= view.scale;
    out.y *= view.scale;

    let scale: f32 = view.resolution.y / 20.0;

    out.x /= scale;
    out.y /= scale;

    return out;
}

@fragment
fn f_main() -> @location(0) vec4<f32> {
    return polygon_view.color;
}
