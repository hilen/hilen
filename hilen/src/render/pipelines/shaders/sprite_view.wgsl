
// Shared by every level shader, glued in front of each one. Must match
// `SpriteView` and `SpriteLight` in `render/shader_data.rs`.

// `color.w` is the intensity.
struct SpriteLight {
    color:    vec4<f32>,
    position: vec2<f32>,
    radius:   f32,
    falloff:  f32,
}

struct SpriteView {
    camera_pos:      vec2<f32>,
    resolution:      vec2<f32>,
    camera_rotation: f32,
    scale:           f32,
    light_count:     u32,
    _padding:        u32,
    ambient:         vec4<f32>,
    lights:          array<SpriteLight, 64>,
}

@group(0) @binding(0)
var<uniform> view: SpriteView;

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

// A level point to clip space, with the camera and the level scale.
fn level_to_clip(world: vec2<f32>, z: f32) -> vec4<f32> {
    var out_pos: vec4<f32> = vec4<f32>(world - view.camera_pos, z, 1.0);

    out_pos *= rotation_z_matrix(view.camera_rotation);

    out_pos.x *= view.resolution.y / view.resolution.x;

    out_pos.x *= view.scale;
    out_pos.y *= view.scale;

    let scale: f32 = view.resolution.y / 20.0;

    out_pos.x /= scale;
    out_pos.y /= scale;

    return out_pos;
}

// The light falling on a level point: the ambient plus every point light
// in reach, capped at white so a lit sprite never goes past its own colors.
fn sprite_light(world: vec2<f32>) -> vec3<f32> {
    var light: vec3<f32> = view.ambient.rgb;
    for (var i: u32 = 0u; i < view.light_count; i++) {
        let lamp = view.lights[i];
        let reach = clamp(1.0 - distance(world, lamp.position) / lamp.radius, 0.0, 1.0);
        light += lamp.color.rgb * lamp.color.w * pow(reach, lamp.falloff);
    }
    return min(light, vec3<f32>(1.0));
}
