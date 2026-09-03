// The sun's depth pass. Every opaque node drawn from the light with the
// same vertex and instance buffers as the main pass, depth only.

@group(0) @binding(0)
var<uniform> sun_view_proj: mat4x4<f32>;

// The frame's joint matrices, the same buffer the main pass reads.
@group(1) @binding(0)
var<storage, read> joints: array<mat4x4<f32>>;

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
    @location(11) joint_base: u32,
}

struct SkinVertex {
    @location(12) joints: vec4<u32>,
    @location(13) weights: vec4<f32>,
}

fn place(pos: vec3<f32>, instance: Instance) -> vec4<f32> {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    return sun_view_proj * model * vec4<f32>(pos, 1.0);
}

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> @builtin(position) vec4<f32> {
    return place(vertex.pos, instance);
}

@vertex
fn v_skinned(vertex: Vertex, instance: Instance, skin: SkinVertex) -> @builtin(position) vec4<f32> {
    let base = instance.joint_base;
    let matrix = skin.weights.x * joints[base + skin.joints.x]
        + skin.weights.y * joints[base + skin.joints.y]
        + skin.weights.z * joints[base + skin.joints.z]
        + skin.weights.w * joints[base + skin.joints.w];
    return place((matrix * vec4<f32>(vertex.pos, 1.0)).xyz, instance);
}
