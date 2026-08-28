
struct SpriteView {
    camera_pos: vec2<f32>,
    resolution: vec2<f32>,
    camera_rotation: f32,
    scale: f32,
    _padding: vec2<u32>,
}

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct TexturedSpriteInstance {
    @location(2) position:   vec2<f32>,
    @location(3) size:       vec2<f32>,
    @location(4) scale:      f32,
    @location(5) rotation:   f32,
    @location(6) z_position: f32,
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

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn v_main(
    model: Vertex,
    instance: TexturedSpriteInstance,
) -> VertexOutput {
    var out_pos: vec4<f32> = vec4<f32>(model.pos, instance.z_position, 1.0);

    out_pos.x *= instance.size.x;
    out_pos.y *= instance.size.y;

    out_pos *= rotation_z_matrix(-instance.rotation);

    out_pos.x += instance.position.x - view.camera_pos.x;
    out_pos.y += instance.position.y - view.camera_pos.y;

    out_pos *=  rotation_z_matrix(view.camera_rotation);

    out_pos.x *= view.resolution.y / view.resolution.x;

    out_pos.x *= instance.scale;
    out_pos.y *= instance.scale;

    out_pos.x *= view.scale;
    out_pos.y *= view.scale;

    let scale: f32 = view.resolution.y / 20.0;

    out_pos.x /= scale;
    out_pos.y /= scale;

    var out: VertexOutput;
    out.pos   = out_pos;
    out.uv = model.uv;
    return out;
}

@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);

    // Bilinear sampling smears a cutout edge across as many screen pixels
    // as the sprite is magnified. Reading the sampled alpha as a distance
    // to the edge and keeping one screen pixel of ramp around its half way
    // line gives the edge the same one pixel coverage the SDF pipelines
    // have, at any scale. A flat alpha has no gradient and is kept as is,
    // so a translucent sprite stays translucent.
    let width: f32 = fwidth(tex.a);
    let sharp: f32 = clamp((tex.a - 0.5) / max(width, 0.0001) + 0.5, 0.0, 1.0);
    let alpha: f32 = select(sharp, tex.a, width == 0.0);

    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(tex.rgb, alpha);
}
