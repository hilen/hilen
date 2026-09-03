// The sun's depth pass. Every opaque node drawn from the light with the
// same vertex and instance buffers as the main pass, depth only.

@group(0) @binding(0)
var<uniform> sun_view_proj: mat4x4<f32>;

struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct Instance {
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
    @location(7) normal0: vec4<f32>,
    @location(8) normal1: vec4<f32>,
    @location(9) normal2: vec4<f32>,
    @location(10) index: u32,
}

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    return sun_view_proj * model * vec4<f32>(vertex.pos, 1.0);
}
