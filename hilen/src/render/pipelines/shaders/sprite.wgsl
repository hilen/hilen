
struct SpriteInstance {
    @location(2) position:   vec2<f32>,
    @location(3) size:       vec2<f32>,
    @location(4) color:      vec4<f32>,
    @location(5) rotation:   f32,
    @location(6) z_position: f32,
}

struct VertexOutput {
    @builtin(position)   pos: vec4<f32>,
          @location(0) color: vec4<f32>,
          @location(1) world: vec2<f32>,
}

@vertex
fn v_main(
    @location(0) model: vec2<f32>,
    instance: SpriteInstance,
) -> VertexOutput {
    let local = (vec4<f32>(model * instance.size, 0.0, 1.0) * rotation_z_matrix(-instance.rotation)).xy;
    let world = local + instance.position;

    var out: VertexOutput;
    out.pos   = level_to_clip(world, instance.z_position);
    out.color = instance.color;
    out.world = world;
    return out;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgb * sprite_light(in.world), in.color.a);
}
