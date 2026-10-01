// The sun's depth pass. Every opaque node drawn from the light with the
// same vertex and instance buffers as the main pass, depth only. A cut
// out node goes through the `cutout` entries, which drop the fragments
// the main pass drops, so it casts the shadow of what is drawn.

@group(0) @binding(0)
var<uniform> sun_view_proj: mat4x4<f32>;

// The part of `MeshInstance` a cut out fragment reads, the same buffer
// and bind the main pass draws the opaque nodes with.
struct MeshInstance {
    model: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    light_count: u32,
    index: u32,
    lights: vec4<u32>,
    normal_scale: f32,
    joint_base: u32,
    emissive: f32,
    cutout: f32,
}

@group(1) @binding(0)
var<storage, read> instances: array<MeshInstance>;

// The frame's joint matrices.
@group(1) @binding(1)
var<storage, read> joints: array<mat4x4<f32>>;

@group(2) @binding(0)
var base_texture: texture_2d<f32>;
@group(2) @binding(1)
var base_sampler: sampler;

struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(14) color: u32,
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

// What the alpha of a cut out fragment is made of, four components
// across the stage boundary.
struct CutoutOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance: u32,
    @location(2) @interpolate(flat) color: u32,
}

fn place(pos: vec3<f32>, instance: Instance) -> vec4<f32> {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    return sun_view_proj * model * vec4<f32>(pos, 1.0);
}

fn skinned(vertex: Vertex, instance: Instance, skin: SkinVertex) -> vec3<f32> {
    let base = instance.joint_base;
    let matrix = skin.weights.x * joints[base + skin.joints.x]
        + skin.weights.y * joints[base + skin.joints.y]
        + skin.weights.z * joints[base + skin.joints.z]
        + skin.weights.w * joints[base + skin.joints.w];
    return (matrix * vec4<f32>(vertex.pos, 1.0)).xyz;
}

fn cutout(pos: vec3<f32>, vertex: Vertex, instance: Instance) -> CutoutOutput {
    var out: CutoutOutput;
    out.pos = place(pos, instance);
    out.uv = vertex.uv;
    out.instance = instance.index;
    out.color = vertex.color;
    return out;
}

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> @builtin(position) vec4<f32> {
    return place(vertex.pos, instance);
}

@vertex
fn v_skinned(vertex: Vertex, instance: Instance, skin: SkinVertex) -> @builtin(position) vec4<f32> {
    return place(skinned(vertex, instance, skin), instance);
}

@vertex
fn v_cutout(vertex: Vertex, instance: Instance) -> CutoutOutput {
    return cutout(vertex.pos, vertex, instance);
}

@vertex
fn v_cutout_skinned(vertex: Vertex, instance: Instance, skin: SkinVertex) -> CutoutOutput {
    return cutout(skinned(vertex, instance, skin), vertex, instance);
}

// The alpha `f_cutout` of the mesh shader tests, term by term.
@fragment
fn f_cutout(in: CutoutOutput) {
    let instance = instances[in.instance];
    let texel = textureSample(base_texture, base_sampler, in.uv);
    let alpha = instance.color.a * unpack4x8unorm(in.color).a * texel.a;
    if alpha < instance.cutout {
        discard;
    }
}
