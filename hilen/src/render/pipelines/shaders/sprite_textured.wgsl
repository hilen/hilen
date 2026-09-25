
struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

// `flags` bit 0 mirrors the image left to right, bit 1 top to bottom.
struct TexturedSpriteInstance {
    @location(2) tint:       vec4<f32>,
    @location(3) position:   vec2<f32>,
    @location(4) size:       vec2<f32>,
    @location(5) scale:      f32,
    @location(6) rotation:   f32,
    @location(7) z_position: f32,
    @location(8) flags:      u32,
}

// Eight float components cross to the fragment stage, the most an A7
// draws, see docs/ios.md.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec2<f32>,
    @location(2) tint: vec4<f32>,
}

@vertex
fn v_main(
    model: Vertex,
    instance: TexturedSpriteInstance,
) -> VertexOutput {
    let local = (vec4<f32>(model.pos * instance.size, 0.0, 1.0) * rotation_z_matrix(-instance.rotation)).xy;

    // The image scale stretches the sprite's distance to the camera too.
    let world = view.camera_pos + (local + instance.position - view.camera_pos) * instance.scale;

    var uv = model.uv;
    if (instance.flags & 1u) != 0u {
        uv.x = 1.0 - uv.x;
    }
    if (instance.flags & 2u) != 0u {
        uv.y = 1.0 - uv.y;
    }

    var out: VertexOutput;
    out.pos   = level_to_clip(world, instance.z_position);
    out.uv    = uv;
    out.world = world;
    out.tint  = instance.tint;
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
    let alpha: f32 = select(sharp, tex.a, width == 0.0) * in.tint.a;

    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(tex.rgb * in.tint.rgb * sprite_light(in.world), alpha);
}
